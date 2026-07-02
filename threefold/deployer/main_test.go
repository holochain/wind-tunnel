package main

import (
	"errors"
	"math/big"
	"testing"

	gsrpcTypes "github.com/centrifuge/go-substrate-rpc-client/v4/types"
	substrate "github.com/threefoldtech/tfchain/clients/tfchain-client-go"
	"github.com/threefoldtech/tfgrid-sdk-go/grid-proxy/pkg/types"
	"github.com/threefoldtech/zosbase/pkg/gridtypes"
)

type fakeNodeDeployabilityChecker struct {
	allowedByNodeID map[uint32]bool
	errByNodeID     map[uint32]error
}

func (c fakeNodeDeployabilityChecker) isOptedOutNodeAllowed(nodeID uint32) (bool, error) {
	if err, ok := c.errByNodeID[nodeID]; ok {
		return false, err
	}
	if allowed, ok := c.allowedByNodeID[nodeID]; ok {
		return allowed, nil
	}
	return true, nil
}

func TestPlacementsForNodesFiltersOptedOutNodes(t *testing.T) {
	cfg := deployConfig{
		vmCPU:    2,
		vmMemGB:  8,
		vmDiskGB: 20,
	}
	nodes := []types.Node{
		testNode(1, 4, 16, 40),
		testNode(2, 8, 32, 80),
		testNode(3, 2, 8, 20),
	}
	checker := fakeNodeDeployabilityChecker{
		allowedByNodeID: map[uint32]bool{
			2: false,
		},
	}

	placements, filtered, err := placementsForNodes("europe", nodes, cfg, checker)
	if err != nil {
		t.Fatalf("placementsForNodes returned error: %v", err)
	}
	if filtered != 1 {
		t.Fatalf("filtered = %d, want 1", filtered)
	}
	if len(placements) != 2 {
		t.Fatalf("len(placements) = %d, want 2", len(placements))
	}

	want := []nodePlacement{
		{nodeID: 1, region: "europe", capacity: 2, byCPU: 2, byMem: 2, byDisk: 2},
		{nodeID: 3, region: "europe", capacity: 1, byCPU: 1, byMem: 1, byDisk: 1},
	}
	for i, got := range placements {
		if got != want[i] {
			t.Fatalf("placements[%d] = %+v, want %+v", i, got, want[i])
		}
	}
}

func TestPlacementsForNodesSkipsDeployabilityLookupErrors(t *testing.T) {
	cfg := deployConfig{
		vmCPU:    2,
		vmMemGB:  8,
		vmDiskGB: 20,
	}
	checker := fakeNodeDeployabilityChecker{
		errByNodeID: map[uint32]error{
			42: errors.New("storage unavailable"),
		},
	}
	nodes := []types.Node{
		testNode(41, 4, 16, 40),
		testNode(42, 4, 16, 40),
		testNode(43, 2, 8, 20),
	}

	placements, filtered, err := placementsForNodes("americas", nodes, cfg, checker)
	if err != nil {
		t.Fatalf("placementsForNodes returned error: %v", err)
	}
	if filtered != 0 {
		t.Fatalf("filtered = %d, want 0", filtered)
	}
	want := []nodePlacement{
		{nodeID: 41, region: "americas", capacity: 2, byCPU: 2, byMem: 2, byDisk: 2},
		{nodeID: 43, region: "americas", capacity: 1, byCPU: 1, byMem: 1, byDisk: 1},
	}
	if len(placements) != len(want) {
		t.Fatalf("len(placements) = %d, want %d", len(placements), len(want))
	}
	for i, got := range placements {
		if got != want[i] {
			t.Fatalf("placements[%d] = %+v, want %+v", i, got, want[i])
		}
	}
}

func TestSnapshotFromBalanceFormatsTFTAndSpendableBalance(t *testing.T) {
	snapshot := snapshotFromBalance("wallet-address", substrate.Balance{
		Free:       testU128(1_000_000_001),
		Reserved:   testU128(20_000_000),
		MiscFrozen: testU128(30_000_000),
		FreeFrozen: testU128(40_000_000),
	})

	if snapshot.address != "wallet-address" {
		t.Fatalf("address = %q, want wallet-address", snapshot.address)
	}
	if snapshot.freeTFT != "100.0000001" {
		t.Fatalf("freeTFT = %q, want 100.0000001", snapshot.freeTFT)
	}
	if snapshot.reservedTFT != "2.0000000" {
		t.Fatalf("reservedTFT = %q, want 2.0000000", snapshot.reservedTFT)
	}
	if snapshot.miscFrozenTFT != "3.0000000" {
		t.Fatalf("miscFrozenTFT = %q, want 3.0000000", snapshot.miscFrozenTFT)
	}
	if snapshot.freeFrozenTFT != "4.0000000" {
		t.Fatalf("freeFrozenTFT = %q, want 4.0000000", snapshot.freeFrozenTFT)
	}
	if snapshot.spendableTFT != "96.0000001" {
		t.Fatalf("spendableTFT = %q, want 96.0000001", snapshot.spendableTFT)
	}
}

func TestSpendableMicroTFTDoesNotGoNegative(t *testing.T) {
	spendable := spendableMicroTFT(substrate.Balance{
		Free:       testU128(10),
		FreeFrozen: testU128(20),
	})
	if spendable.Sign() != 0 {
		t.Fatalf("spendable = %s, want 0", spendable.String())
	}
}

func testNode(nodeID int, cru uint64, mruGiB uint64, sruGiB uint64) types.Node {
	return types.Node{
		NodeID: nodeID,
		TotalResources: types.Capacity{
			CRU: cru,
			MRU: gridtypes.Unit(mruGiB) * gridtypes.Gigabyte,
			SRU: gridtypes.Unit(sruGiB) * gridtypes.Gigabyte,
		},
	}
}

func testU128(value int64) gsrpcTypes.U128 {
	return gsrpcTypes.U128{Int: big.NewInt(value)}
}
