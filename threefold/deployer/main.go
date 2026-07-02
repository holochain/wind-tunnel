package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"math"
	"math/big"
	"net"
	"os"
	"regexp"
	"sort"
	"strings"
	"sync"

	gsrpcTypes "github.com/centrifuge/go-substrate-rpc-client/v4/types"
	substrate "github.com/threefoldtech/tfchain/clients/tfchain-client-go"
	"github.com/threefoldtech/tfgrid-sdk-go/grid-client/deployer"
	"github.com/threefoldtech/tfgrid-sdk-go/grid-client/workloads"
	"github.com/threefoldtech/tfgrid-sdk-go/grid-client/zos"
	"github.com/threefoldtech/tfgrid-sdk-go/grid-proxy/pkg/types"
)

// Operator Notes:
// - This tool deploys wind-tunnel runner VMs dynamically based on available node capacity.
// - Placement order follows --regions exactly (default: europe,americas).
// - Capacity is computed from per-VM minimum CPU, memory, and disk requirements.
// - Each node deploys independently: one network and one VM deployment per node.
// - Deployments run concurrently with --max-concurrency.
// - Deployment succeeds once target VMs are reached, even if some nodes fail.

const (
	gib                = uint64(1024 * 1024 * 1024)
	maxPrefixLen       = 18
	defaultRegionOrder = "europe,americas"
	defaultConcurrency = 6

	deploymentNameSuffix = "_dl_"
	vmNameSuffix         = "_vm_"
	networkNameSuffix    = "_net_"
	githubOutputFormat   = "github"
	textOutputFormat     = "text"
)

var prefixPattern = regexp.MustCompile(`^[A-Za-z0-9_]+$`)

type deployConfig struct {
	mnemonic       string
	network        string
	targetVMs      int
	vmCPU          uint64
	vmMemGB        uint64
	vmDiskGB       uint64
	flist          string
	entrypoint     string
	prefix         string
	maxConcurrency int
	regionOrder    []string
}

type accountSnapshotConfig struct {
	mnemonic string
	network  string
	format   string
}

type accountSnapshot struct {
	address       string
	freeTFT       string
	reservedTFT   string
	miscFrozenTFT string
	freeFrozenTFT string
	spendableTFT  string
}

type nodePlacement struct {
	nodeID   uint32
	region   string
	capacity int
	byCPU    int64
	byMem    int64
	byDisk   int64
}

type deployResult struct {
	workerID int
	nodeID   uint32
	region   string
	assigned int
	err      error
}

type deployJob struct {
	nodePlacement
	assigned int
}

type substrateStorageGetter interface {
	GetClient() (substrate.Conn, substrate.Meta, error)
}

type nodeDeployabilityChecker interface {
	isOptedOutNodeAllowed(nodeID uint32) (bool, error)
}

type tfchainNodeDeployabilityChecker struct {
	storageGetter      substrateStorageGetter
	accountID          substrate.AccountID
	isAllowedTwinAdmin bool
}

func main() {
	if err := run(os.Args[1:]); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return
		}
		log.Fatal(err)
	}
}

func run(args []string) error {
	if len(args) > 0 && args[0] == "account-snapshot" {
		cfg, err := parseAccountSnapshotCliArgs(args[1:])
		if err != nil {
			return err
		}
		return printAccountSnapshot(cfg)
	}

	cfg, err := parseCliArgs(args)
	if err != nil {
		return err
	}
	return deploy(cfg)
}

// Parse and validate cli
func parseCliArgs(args []string) (deployConfig, error) {
	fs := flag.NewFlagSet("deploy", flag.ContinueOnError)
	cfg := deployConfig{}
	regionsCSV := defaultRegionOrder

	// Parse cli args
	fs.StringVar(&cfg.mnemonic, "mnemonic", "", "ThreeFold wallet mnemonic")
	fs.StringVar(&cfg.network, "network", "main", "ThreeFold network")
	fs.IntVar(&cfg.targetVMs, "target-vms", 0, "Total number of VMs to deploy")
	fs.Uint64Var(&cfg.vmCPU, "vm-cpu", 2, "Per-VM CPU requirement")
	fs.Uint64Var(&cfg.vmMemGB, "vm-memory-gb", 8, "Per-VM memory requirement in GiB")
	fs.Uint64Var(&cfg.vmDiskGB, "vm-disk-gb", 20, "Per-VM rootfs disk requirement in GiB")
	fs.StringVar(&cfg.flist, "flist", "", "VM flist URL")
	fs.StringVar(&cfg.entrypoint, "entrypoint", "/entrypoint.sh", "VM entrypoint")
	fs.StringVar(&cfg.prefix, "prefix", "wt_runner", "Deployment and VM name prefix (A-Z, a-z, 0-9, underscore)")
	fs.IntVar(&cfg.maxConcurrency, "max-concurrency", defaultConcurrency, "Maximum concurrent node deployments")
	fs.StringVar(&regionsCSV, "regions", defaultRegionOrder, "Comma-separated placement region order")

	if err := fs.Parse(args); err != nil {
		return cfg, err
	}
	// Parse cli arg `region` into list of regions
	parts := strings.Split(regionsCSV, ",")
	regionOrder := make([]string, 0, len(parts))
	for _, part := range parts {
		region := strings.ToLower(strings.TrimSpace(part))
		if region == "" {
			continue
		}
		if !prefixPattern.MatchString(region) {
			return cfg, fmt.Errorf("invalid region '%s' in --regions (expected alphanumeric/underscore)", part)
		}
		regionOrder = append(regionOrder, region)
	}
	if len(regionOrder) == 0 {
		return cfg, fmt.Errorf("--regions must contain at least one region")
	}

	cfg.regionOrder = regionOrder

	// Validate cli args
	if strings.TrimSpace(cfg.mnemonic) == "" {
		return cfg, fmt.Errorf("--mnemonic is required")
	}
	if cfg.targetVMs <= 0 {
		return cfg, fmt.Errorf("--target-vms must be > 0")
	}
	if cfg.vmCPU == 0 || cfg.vmMemGB == 0 || cfg.vmDiskGB == 0 {
		return cfg, fmt.Errorf("--vm-cpu, --vm-memory-gb, and --vm-disk-gb must all be > 0")
	}
	if cfg.vmCPU > math.MaxUint8 {
		return cfg, fmt.Errorf("--vm-cpu must be <= %d", math.MaxUint8)
	}
	if strings.TrimSpace(cfg.flist) == "" {
		return cfg, fmt.Errorf("--flist is required")
	}
	if len(cfg.prefix) == 0 || len(cfg.prefix) > maxPrefixLen {
		return cfg, fmt.Errorf("--prefix must be between 1 and %d characters", maxPrefixLen)
	}
	if !prefixPattern.MatchString(cfg.prefix) {
		return cfg, fmt.Errorf("--prefix must match %s", prefixPattern.String())
	}
	if cfg.maxConcurrency <= 0 {
		return cfg, fmt.Errorf("--max-concurrency must be > 0")
	}

	return cfg, nil
}

func parseAccountSnapshotCliArgs(args []string) (accountSnapshotConfig, error) {
	fs := flag.NewFlagSet("account-snapshot", flag.ContinueOnError)
	cfg := accountSnapshotConfig{
		network: "main",
		format:  textOutputFormat,
	}

	fs.StringVar(&cfg.mnemonic, "mnemonic", "", "ThreeFold wallet mnemonic")
	fs.StringVar(&cfg.network, "network", "main", "ThreeFold network")
	fs.StringVar(&cfg.format, "format", textOutputFormat, "Output format: text or github")

	if err := fs.Parse(args); err != nil {
		return cfg, err
	}
	if strings.TrimSpace(cfg.mnemonic) == "" {
		return cfg, fmt.Errorf("--mnemonic is required")
	}
	if cfg.format != textOutputFormat && cfg.format != githubOutputFormat {
		return cfg, fmt.Errorf("--format must be %q or %q", textOutputFormat, githubOutputFormat)
	}

	return cfg, nil
}

// Orchestrates deployment of nodes, vms and networks as follows:
// - Query for nodes that meet minimum capacity requirements, one region at a time, ordered by `regions` list
// - Calculate total number of VMs that could be deployed to those nodes, based on our VM hardware requirements
// - Spawn threads to actually execute deployments in parallal
//
// A single network is deployed per node
func deploy(cfg deployConfig) error {
	ctx := context.Background()

	pluginOpts := []deployer.PluginOpt{deployer.WithNetwork(cfg.network)}
	pluginOpts = append(pluginOpts, deployer.WithLogs())
	log.Printf("verbose mode enabled")
	log.Printf("deploy config: network=%s target_vms=%d vm_cpu=%d vm_mem_gb=%d vm_disk_gb=%d regions=%s prefix=%s max_concurrency=%d", cfg.network, cfg.targetVMs, cfg.vmCPU, cfg.vmMemGB, cfg.vmDiskGB, strings.Join(cfg.regionOrder, ","), cfg.prefix, cfg.maxConcurrency)

	// Create threefold client
	tf, err := deployer.NewTFPluginClient(cfg.mnemonic, pluginOpts...)
	if err != nil {
		return fmt.Errorf("create tf plugin client: %w", err)
	}
	defer tf.Close()

	checker, err := newNodeDeployabilityChecker(tf)
	if err != nil {
		return err
	}

	// Determine the number of VM placements available per region
	candidates := make([]nodePlacement, 0)
	totalCandidateCapacity := 0
	for _, region := range cfg.regionOrder {
		placements, err := placementsForRegion(ctx, tf, region, cfg, checker)
		if err != nil {
			return err
		}
		regionCapacity := 0
		for _, p := range placements {
			regionCapacity += p.capacity
		}
		totalCandidateCapacity += regionCapacity
		log.Printf("region=%s capacity_vms=%d", region, regionCapacity)
		candidates = append(candidates, placements...)
	}

	// Check if there is enough nodes & VM capacity for our hardware requirements and number of VMs
	if len(candidates) == 0 {
		return fmt.Errorf("no candidate nodes available")
	}

	log.Printf(
		"preflight capacity target_vms=%d candidate_capacity_vms=%d candidate_nodes=%d regions=%s",
		cfg.targetVMs,
		totalCandidateCapacity,
		len(candidates),
		strings.Join(cfg.regionOrder, ","),
	)

	if totalCandidateCapacity < cfg.targetVMs {
		return fmt.Errorf(
			"insufficient preflight capacity: requested %d VMs, estimated %d slots across %d candidate nodes (short by %d)",
			cfg.targetVMs,
			totalCandidateCapacity,
			len(candidates),
			cfg.targetVMs-totalCandidateCapacity,
		)
	}

	// Deploy the nodes, networks, and VMs
	deployedVMs, attemptedNodes, successfulNodes, failures := deployConcurrently(ctx, tf, cfg, candidates)
	remaining := cfg.targetVMs - deployedVMs
	if remaining < 0 {
		remaining = 0
	}

	log.Printf("deploy summary target_vms=%d deployed_vms=%d attempted_nodes=%d successful_nodes=%d failed_nodes=%d remaining=%d", cfg.targetVMs, deployedVMs, attemptedNodes, successfulNodes, len(failures), remaining)
	for _, failure := range failures {
		log.Printf("failure detail: %s", failure)
	}

	if deployedVMs < cfg.targetVMs {
		return fmt.Errorf("could not reach target size: requested %d VMs, deployed %d, short by %d", cfg.targetVMs, deployedVMs, cfg.targetVMs-deployedVMs)
	}
	return nil
}

// Schedules node deployments concurrenently up to the desired count, or there is no more availability.
func deployConcurrently(ctx context.Context, tf deployer.TFPluginClient, cfg deployConfig, candidates []nodePlacement) (int, int, int, []string) {
	jobs := make(chan deployJob)
	results := make(chan deployResult, cfg.maxConcurrency)

	var workerWG sync.WaitGroup
	for workerID := 1; workerID <= cfg.maxConcurrency; workerID++ {
		workerWG.Add(1)
		go func(id int) {
			defer workerWG.Done()
			for job := range jobs {
				log.Printf("[worker-%d] start region=%s node_id=%d vms=%d", id, job.region, job.nodeID, job.assigned)

				err := deployNode(ctx, tf, cfg, job.region, job.nodeID, job.assigned)
				results <- deployResult{workerID: id, nodeID: job.nodeID, region: job.region, assigned: job.assigned, err: err}
			}
		}(workerID)
	}

	deployedVMs := 0
	attemptedNodes := 0
	successfulNodes := 0
	inFlightAssigned := 0
	inFlightWorkers := 0
	failures := make([]string, 0)
	next := 0

	scheduleNext := func() bool {
		if inFlightWorkers >= cfg.maxConcurrency {
			return false
		}
		if next >= len(candidates) {
			return false
		}
		need := cfg.targetVMs - (deployedVMs + inFlightAssigned)
		if need <= 0 {
			return false
		}

		candidate := candidates[next]
		next++
		assign := candidate.capacity
		if assign > need {
			assign = need
		}
		if assign <= 0 {
			return true
		}

		attemptedNodes++
		inFlightAssigned += assign
		inFlightWorkers++
		jobs <- deployJob{nodePlacement: candidate, assigned: assign}

		log.Printf("[scheduler] scheduled region=%s node_id=%d vms=%d in_flight_workers=%d in_flight_assigned=%d", candidate.region, candidate.nodeID, assign, inFlightWorkers, inFlightAssigned)
		return true
	}

	for {
		if !scheduleNext() {
			break
		}
	}

	for inFlightWorkers > 0 {
		res := <-results
		inFlightWorkers--
		inFlightAssigned -= res.assigned
		if res.err != nil {
			failures = append(failures, fmt.Sprintf("region=%s node_id=%d err=%v", res.region, res.nodeID, res.err))
			log.Printf("[worker-%d] failed region=%s node_id=%d vms=%d err=%v", res.workerID, res.region, res.nodeID, res.assigned, res.err)
		} else {
			deployedVMs += res.assigned
			successfulNodes++
			log.Printf("[worker-%d] succeeded region=%s node_id=%d vms=%d deployed_total=%d", res.workerID, res.region, res.nodeID, res.assigned, deployedVMs)
		}
		log.Printf("[scheduler] in_flight_workers=%d in_flight_assigned=%d", inFlightWorkers, inFlightAssigned)

		for {
			if !scheduleNext() {
				break
			}
		}
	}

	close(jobs)
	workerWG.Wait()
	close(results)
	return deployedVMs, attemptedNodes, successfulNodes, failures
}

// Deploys a network and all VMs for a single node
func deployNode(ctx context.Context, tf deployer.TFPluginClient, cfg deployConfig, region string, nodeID uint32, vmCount int) error {
	networkName := fmt.Sprintf("%s%s%d", cfg.prefix, networkNameSuffix, nodeID)
	nw := workloads.ZNet{
		Name:        networkName,
		Description: "wind-tunnel-runner threefold",
		Nodes:       []uint32{nodeID},
		IPRange: zos.IPNet{IPNet: net.IPNet{
			IP:   net.IPv4(10, 20, 0, 0),
			Mask: net.CIDRMask(16, 32),
		}},
	}

	if err := tf.NetworkDeployer.Deploy(ctx, &nw); err != nil {
		return fmt.Errorf("deploy network %s (region=%s node=%d): %w", nw.Name, region, nodeID, err)
	}

	vms := make([]workloads.VM, 0, vmCount)
	for i := 0; i < vmCount; i++ {
		vmName := fmt.Sprintf("%s%s%d_%d", cfg.prefix, vmNameSuffix, nodeID, i)
		vms = append(vms, workloads.VM{
			Name:         vmName,
			NodeID:       nodeID,
			NetworkName:  nw.Name,
			CPU:          uint8(cfg.vmCPU),
			MemoryMB:     cfg.vmMemGB * 1024,
			RootfsSizeMB: cfg.vmDiskGB * 1024,
			Flist:        cfg.flist,
			Entrypoint:   cfg.entrypoint,
			Planetary:    true,
			EnvVars: map[string]string{
				"SSH_KEY": "unused-key",
			},
		})
	}

	dlName := fmt.Sprintf("%s%s%d", cfg.prefix, deploymentNameSuffix, nodeID)
	dl := workloads.NewDeployment(dlName, nodeID, "", nil, nw.Name, nil, nil, vms, nil, nil, nil)
	if err := tf.DeploymentDeployer.Deploy(ctx, &dl); err != nil {
		return fmt.Errorf("deploy vm deployment %s (region=%s node=%d): %w", dlName, region, nodeID, err)
	}

	return nil
}

// Filters eligible nodes for a region and computes per-node
// VM capacity based on the configured hardware requirements.
//
// Returns potential nodes sorted by descending capacity, then by node ID.
func placementsForRegion(ctx context.Context, tf deployer.TFPluginClient, region string, cfg deployConfig, checker nodeDeployabilityChecker) ([]nodePlacement, error) {
	regionFilter := region
	freeMRU := cfg.vmMemGB * gib
	freeSRU := cfg.vmDiskGB * gib

	filter := types.NodeFilter{
		Status:  []string{"up"},
		Region:  &regionFilter,
		FreeMRU: &freeMRU,
		FreeSRU: &freeSRU,
	}

	nodes, err := deployer.FilterNodes(ctx, tf, filter, nil, nil, nil)
	if err != nil {
		if strings.Contains(err.Error(), "could not find enough nodes") {
			return nil, nil
		}
		return nil, fmt.Errorf("filter %s nodes: %w", region, err)
	}
	log.Printf("region=%s candidates=%d (status=up free_mru>=%dGiB free_sru>=%dGiB)", region, len(nodes), cfg.vmMemGB, cfg.vmDiskGB)

	placements, filteredOptOutNodes, err := placementsForNodes(region, nodes, cfg, checker)
	if err != nil {
		return nil, err
	}

	// Report only the nodes that passed the grid-proxy capacity filter but were
	// excluded because TFChain would reject contract creation for this wallet.
	if filteredOptOutNodes > 0 {
		log.Printf("region=%s filtered_opted_out_nodes=%d", region, filteredOptOutNodes)
	}

	return placements, nil
}

func placementsForNodes(region string, nodes []types.Node, cfg deployConfig, checker nodeDeployabilityChecker) ([]nodePlacement, int, error) {
	placements := make([]nodePlacement, 0, len(nodes))
	filteredOptOutNodes := 0
	for _, n := range nodes {
		allowed, err := checker.isOptedOutNodeAllowed(uint32(n.NodeID))
		if err != nil {
			log.Printf("region=%s node_id=%d skipped: failed to check V3 billing opt-out status: %v", region, n.NodeID, err)
			continue
		}
		if !allowed {
			filteredOptOutNodes++
			// V3 billing opt-out is a migration/admin path. Non-admin wallets
			// cannot deploy on these nodes, so preflight must not count their
			// capacity as usable.
			log.Printf("region=%s node_id=%d skipped: node is V3 billing opted-out and wallet is not an allowed twin admin", region, n.NodeID)
			continue
		}

		cap, byCPU, byMem, byDisk := capacityForNode(n, cfg)
		if cap <= 0 {
			continue
		}
		placements = append(placements, nodePlacement{
			nodeID:   uint32(n.NodeID),
			region:   region,
			capacity: cap,
			byCPU:    byCPU,
			byMem:    byMem,
			byDisk:   byDisk,
		})
	}

	sort.Slice(placements, func(i, j int) bool {
		if placements[i].capacity == placements[j].capacity {
			return placements[i].nodeID < placements[j].nodeID
		}
		return placements[i].capacity > placements[j].capacity
	})

	for i, p := range placements {
		if i >= 15 {
			break
		}
		log.Printf("region=%s node_id=%d node_capacity=%d limits(cpu=%d mem=%d disk=%d)", region, p.nodeID, p.capacity, p.byCPU, p.byMem, p.byDisk)
	}

	return placements, filteredOptOutNodes, nil
}

// Calculate the maxmimum number of VMs that can be deployed to a single node,
// based on the VM targeted hardware and node hardware
func capacityForNode(n types.Node, cfg deployConfig) (int, int64, int64, int64) {
	freeCRU := int64(n.TotalResources.CRU) - int64(n.UsedResources.CRU)
	freeMRU := int64(n.TotalResources.MRU) - int64(n.UsedResources.MRU)
	freeSRU := int64(n.TotalResources.SRU) - int64(n.UsedResources.SRU)
	if freeCRU <= 0 || freeMRU <= 0 || freeSRU <= 0 {
		return 0, 0, 0, 0
	}

	byCPU := freeCRU / int64(cfg.vmCPU)
	byMem := freeMRU / int64(cfg.vmMemGB*gib)
	byDisk := freeSRU / int64(cfg.vmDiskGB*gib)

	minCap := byCPU
	if byMem < minCap {
		minCap = byMem
	}
	if byDisk < minCap {
		minCap = byDisk
	}
	if minCap <= 0 {
		return 0, byCPU, byMem, byDisk
	}
	return int(minCap), byCPU, byMem, byDisk
}

func newNodeDeployabilityChecker(tf deployer.TFPluginClient) (nodeDeployabilityChecker, error) {
	storageGetter, ok := tf.SubstrateConn.(substrateStorageGetter)
	if !ok {
		return nil, fmt.Errorf("substrate connection does not expose storage access")
	}

	accountID, err := substrate.FromAddress(tf.Identity.Address())
	if err != nil {
		return nil, fmt.Errorf("decode deployer account address: %w", err)
	}

	checker := &tfchainNodeDeployabilityChecker{
		storageGetter: storageGetter,
		accountID:     accountID,
	}

	checker.isAllowedTwinAdmin, err = checker.fetchAllowedTwinAdmin()
	if err != nil {
		return nil, fmt.Errorf("check allowed twin admin status: %w", err)
	}
	// This reports whether TFChain lists the deployer wallet as an allowed twin
	// admin. Only those wallets may create contracts on nodes that farmers have
	// opted out of normal V3 billing for migration.
	if checker.isAllowedTwinAdmin {
		log.Printf("deployer wallet is an allowed twin admin; V3 billing opted-out ThreeFold nodes are allowed")
	} else {
		log.Printf("deployer wallet is not an allowed twin admin; V3 billing opted-out ThreeFold nodes will be skipped")
	}

	return checker, nil
}

func (c *tfchainNodeDeployabilityChecker) isOptedOutNodeAllowed(nodeID uint32) (bool, error) {
	optedOut, err := c.nodeHasV3BillingOptOut(nodeID)
	if err != nil {
		return false, err
	}
	return !optedOut || c.isAllowedTwinAdmin, nil
}

func (c *tfchainNodeDeployabilityChecker) nodeHasV3BillingOptOut(nodeID uint32) (bool, error) {
	cl, meta, err := c.storageGetter.GetClient()
	if err != nil {
		return false, err
	}

	nodeIDBytes, err := substrate.Encode(nodeID)
	if err != nil {
		return false, fmt.Errorf("encode node id: %w", err)
	}
	key, err := gsrpcTypes.CreateStorageKey(meta, "TfgridModule", "NodeV3BillingOptOut", nodeIDBytes)
	if err != nil {
		return false, fmt.Errorf("create NodeV3BillingOptOut storage key: %w", err)
	}

	var optedOutAt gsrpcTypes.U64
	ok, err := cl.RPC.State.GetStorageLatest(key, &optedOutAt)
	if err != nil {
		return false, fmt.Errorf("query NodeV3BillingOptOut storage: %w", err)
	}
	return ok, nil
}

func (c *tfchainNodeDeployabilityChecker) fetchAllowedTwinAdmin() (bool, error) {
	cl, meta, err := c.storageGetter.GetClient()
	if err != nil {
		return false, err
	}

	key, err := gsrpcTypes.CreateStorageKey(meta, "TfgridModule", "AllowedTwinAdmins")
	if err != nil {
		return false, fmt.Errorf("create AllowedTwinAdmins storage key: %w", err)
	}

	var admins []substrate.AccountID
	ok, err := cl.RPC.State.GetStorageLatest(key, &admins)
	if err != nil {
		return false, fmt.Errorf("query AllowedTwinAdmins storage: %w", err)
	}
	if !ok {
		return false, nil
	}

	for _, admin := range admins {
		if admin == c.accountID {
			return true, nil
		}
	}
	return false, nil
}

func printAccountSnapshot(cfg accountSnapshotConfig) error {
	pluginOpts := []deployer.PluginOpt{deployer.WithNetwork(cfg.network)}
	tf, err := deployer.NewTFPluginClient(cfg.mnemonic, pluginOpts...)
	if err != nil {
		return fmt.Errorf("create tf plugin client: %w", err)
	}
	defer tf.Close()

	balance, err := tf.SubstrateConn.GetBalance(tf.Identity)
	if err != nil {
		return fmt.Errorf("get account balance: %w", err)
	}

	snapshot := snapshotFromBalance(tf.Identity.Address(), balance)
	switch cfg.format {
	case githubOutputFormat:
		fmt.Printf("address=%s\n", snapshot.address)
		fmt.Printf("free_tft=%s\n", snapshot.freeTFT)
		fmt.Printf("reserved_tft=%s\n", snapshot.reservedTFT)
		fmt.Printf("misc_frozen_tft=%s\n", snapshot.miscFrozenTFT)
		fmt.Printf("free_frozen_tft=%s\n", snapshot.freeFrozenTFT)
		fmt.Printf("spendable_tft=%s\n", snapshot.spendableTFT)
	case textOutputFormat:
		fmt.Printf("address: %s\n", snapshot.address)
		fmt.Printf("free_tft: %s\n", snapshot.freeTFT)
		fmt.Printf("reserved_tft: %s\n", snapshot.reservedTFT)
		fmt.Printf("misc_frozen_tft: %s\n", snapshot.miscFrozenTFT)
		fmt.Printf("free_frozen_tft: %s\n", snapshot.freeFrozenTFT)
		fmt.Printf("spendable_tft: %s\n", snapshot.spendableTFT)
	default:
		return fmt.Errorf("unsupported output format %q", cfg.format)
	}

	return nil
}

func snapshotFromBalance(address string, balance substrate.Balance) accountSnapshot {
	return accountSnapshot{
		address:       address,
		freeTFT:       formatTFT(balance.Free),
		reservedTFT:   formatTFT(balance.Reserved),
		miscFrozenTFT: formatTFT(balance.MiscFrozen),
		freeFrozenTFT: formatTFT(balance.FreeFrozen),
		spendableTFT:  formatMicroTFT(spendableMicroTFT(balance)),
	}
}

func spendableMicroTFT(balance substrate.Balance) *big.Int {
	free := u128Int(balance.Free)
	frozen := u128Int(balance.MiscFrozen)
	if freeFrozen := u128Int(balance.FreeFrozen); freeFrozen.Cmp(frozen) > 0 {
		frozen = freeFrozen
	}

	spendable := new(big.Int).Sub(free, frozen)
	if spendable.Sign() < 0 {
		return big.NewInt(0)
	}
	return spendable
}

func formatTFT(amount gsrpcTypes.U128) string {
	return formatMicroTFT(u128Int(amount))
}

func formatMicroTFT(amount *big.Int) string {
	unitsPerTFT := big.NewInt(substrate.TFT)
	whole, fractional := new(big.Int), new(big.Int)
	whole.QuoRem(amount, unitsPerTFT, fractional)
	fractionalText := fractional.String()
	if len(fractionalText) < 7 {
		fractionalText = strings.Repeat("0", 7-len(fractionalText)) + fractionalText
	}
	return fmt.Sprintf("%s.%s", whole.String(), fractionalText)
}

func u128Int(amount gsrpcTypes.U128) *big.Int {
	if amount.Int == nil {
		return big.NewInt(0)
	}
	return new(big.Int).Set(amount.Int)
}
