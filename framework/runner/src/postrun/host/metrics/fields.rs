use serde::{Deserialize, Serialize};
use wind_tunnel_instruments::prelude::ReportMetric;

/// Trait for reporting fields of host metrics.
pub trait ReportFields {
    /// Returns the associated [`ReportMetric`] for this host metrics fields.
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric;
}

/// Tags for [`super::HostMetrics`] fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum HostMetricsFields {
    /// [`super::HostMetricsName::Cpu`] [`super::HostMetrics`].
    Cpu(CpuMetrics),
    /// [`super::HostMetricsName::Disk`] [`super::HostMetrics`].
    Disk(DiskMetrics),
    /// [`super::HostMetricsName::Diskio`] [`super::HostMetrics`].
    Diskio(DiskioMetrics),
    /// [`super::HostMetricsName::Kernel`] [`super::HostMetrics`].
    Kernel(KernelMetrics),
    /// [`super::HostMetricsName::Mem`] [`super::HostMetrics`].
    Mem(MemMetrics),
    /// [`super::HostMetricsName::Netstat`] [`super::HostMetrics`].
    Netstat(NetstatMetrics),
    /// [`super::HostMetricsName::Net`] [`super::HostMetrics`].
    Net(NetMetrics),
    /// [`super::HostMetricsName::Processes`] [`super::HostMetrics`].
    Processes(ProcessesMetrics),
    /// [`super::HostMetricsName::System`] [`super::HostMetrics`].
    System(SystemMetrics),
    /// [`super::HostMetricsName::Swap`] [`super::HostMetrics`].
    Swap(SwapMetrics),
}

impl ReportFields for HostMetricsFields {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        match self {
            Self::Cpu(cpu) => cpu.report_fields(metric),
            Self::Disk(disk) => disk.report_fields(metric),
            Self::Diskio(diskio) => diskio.report_fields(metric),
            Self::Kernel(kernel) => kernel.report_fields(metric),
            Self::Mem(mem) => mem.report_fields(metric),
            Self::Net(net) => net.report_fields(metric),
            Self::Netstat(netstat) => netstat.report_fields(metric),
            Self::Processes(processes) => processes.report_fields(metric),
            Self::System(system) => system.report_fields(metric),
            Self::Swap(swap) => swap.report_fields(metric),
        }
    }
}

/// [`super::HostMetricsName::Cpu`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CpuMetrics {
    pub usage_guest: f64,
    pub usage_guest_nice: f64,
    pub usage_idle: f64,
    pub usage_iowait: f64,
    pub usage_irq: f64,
    pub usage_nice: f64,
    pub usage_softirq: f64,
    pub usage_steal: f64,
    pub usage_system: f64,
    pub usage_user: f64,
}

impl ReportFields for CpuMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("usage_guest", self.usage_guest)
            .with_field("usage_guest_nice", self.usage_guest_nice)
            .with_field("usage_idle", self.usage_idle)
            .with_field("usage_iowait", self.usage_iowait)
            .with_field("usage_irq", self.usage_irq)
            .with_field("usage_nice", self.usage_nice)
            .with_field("usage_softirq", self.usage_softirq)
            .with_field("usage_steal", self.usage_steal)
            .with_field("usage_system", self.usage_system)
            .with_field("usage_user", self.usage_user)
    }
}

/// [`super::HostMetricsName::Disk`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiskMetrics {
    pub free: i64,
    pub inodes_free: i64,
    pub inodes_total: i64,
    pub inodes_used: i64,
    pub inodes_used_percent: f64,
    pub total: i64,
    pub used: i64,
    pub used_percent: f64,
}

impl ReportFields for DiskMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("free", self.free)
            .with_field("inodes_free", self.inodes_free)
            .with_field("inodes_total", self.inodes_total)
            .with_field("inodes_used", self.inodes_used)
            .with_field("inodes_used_percent", self.inodes_used_percent)
            .with_field("total", self.total)
            .with_field("used", self.used)
            .with_field("used_percent", self.used_percent)
    }
}

/// [`super::HostMetricsName::Diskio`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiskioMetrics {
    pub io_time: i64,
    pub iops_in_progress: i64,
    pub merged_reads: i64,
    pub merged_writes: i64,
    pub read_bytes: i64,
    pub read_time: i64,
    pub reads: i64,
    pub weighted_io_time: i64,
    pub write_bytes: i64,
    pub write_time: i64,
    pub writes: i64,
}

impl ReportFields for DiskioMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("io_time", self.io_time)
            .with_field("iops_in_progress", self.iops_in_progress)
            .with_field("merged_reads", self.merged_reads)
            .with_field("merged_writes", self.merged_writes)
            .with_field("read_bytes", self.read_bytes)
            .with_field("read_time", self.read_time)
            .with_field("reads", self.reads)
            .with_field("weighted_io_time", self.weighted_io_time)
            .with_field("write_bytes", self.write_bytes)
            .with_field("write_time", self.write_time)
            .with_field("writes", self.writes)
    }
}

/// [`super::HostMetricsName::Kernel`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KernelMetrics {
    pub boot_time: i64,
    pub context_switches: i64,
    pub entropy_avail: i64,
    pub interrupts: i64,
    pub processes_forked: i64,
}

impl ReportFields for KernelMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("boot_time", self.boot_time)
            .with_field("context_switches", self.context_switches)
            .with_field("entropy_avail", self.entropy_avail)
            .with_field("interrupts", self.interrupts)
            .with_field("processes_forked", self.processes_forked)
    }
}

/// [`super::HostMetricsName::Mem`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MemMetrics {
    pub active: i64,
    pub available: i64,
    pub available_percent: f64,
    pub buffered: i64,
    pub cached: i64,
    pub commit_limit: i64,
    pub committed_as: i64,
    pub dirty: i64,
    pub free: i64,
    pub high_free: i64,
    pub high_total: i64,
    pub huge_page_size: i64,
    pub huge_pages_free: i64,
    pub huge_pages_total: i64,
    pub inactive: i64,
    pub low_free: i64,
    pub low_total: i64,
    pub mapped: i64,
    pub page_tables: i64,
    pub shared: i64,
    pub slab: i64,
    pub sreclaimable: i64,
    pub sunreclaim: i64,
    pub swap_cached: i64,
    pub swap_free: i64,
    pub swap_total: i64,
    pub total: i64,
    pub used: i64,
    pub used_percent: f64,
    pub vmalloc_chunk: i64,
    pub vmalloc_total: i64,
    pub vmalloc_used: i64,
    pub write_back: i64,
    pub write_back_tmp: i64,
}

impl ReportFields for MemMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("active", self.active)
            .with_field("available", self.available)
            .with_field("available_percent", self.available_percent)
            .with_field("buffered", self.buffered)
            .with_field("cached", self.cached)
            .with_field("commit_limit", self.commit_limit)
            .with_field("committed_as", self.committed_as)
            .with_field("dirty", self.dirty)
            .with_field("free", self.free)
            .with_field("high_free", self.high_free)
            .with_field("high_total", self.high_total)
            .with_field("huge_page_size", self.huge_page_size)
            .with_field("huge_pages_free", self.huge_pages_free)
            .with_field("huge_pages_total", self.huge_pages_total)
            .with_field("inactive", self.inactive)
            .with_field("low_free", self.low_free)
            .with_field("low_total", self.low_total)
            .with_field("mapped", self.mapped)
            .with_field("page_tables", self.page_tables)
            .with_field("shared", self.shared)
            .with_field("slab", self.slab)
            .with_field("sreclaimable", self.sreclaimable)
            .with_field("sunreclaim", self.sunreclaim)
            .with_field("swap_cached", self.swap_cached)
            .with_field("swap_free", self.swap_free)
            .with_field("swap_total", self.swap_total)
            .with_field("total", self.total)
            .with_field("used", self.used)
            .with_field("used_percent", self.used_percent)
            .with_field("vmalloc_chunk", self.vmalloc_chunk)
            .with_field("vmalloc_total", self.vmalloc_total)
            .with_field("vmalloc_used", self.vmalloc_used)
            .with_field("write_back", self.write_back)
            .with_field("write_back_tmp", self.write_back_tmp)
    }
}

/// [`super::HostMetricsName::Net`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetMetrics {
    pub bytes_recv: Option<i64>,
    pub bytes_sent: Option<i64>,
    pub drop_in: Option<i64>,
    pub drop_out: Option<i64>,
    pub err_in: Option<i64>,
    pub err_out: Option<i64>,
    pub icmp_inaddrmaskreps: Option<i64>,
    pub icmp_inaddrmasks: Option<i64>,
    pub icmp_incsumerrors: Option<i64>,
    pub icmp_indestunreachs: Option<i64>,
    pub icmp_inechoreps: Option<i64>,
    pub icmp_inechos: Option<i64>,
    pub icmp_inerrors: Option<i64>,
    pub icmp_inmsgs: Option<i64>,
    pub icmp_inparmprobs: Option<i64>,
    pub icmp_inredirects: Option<i64>,
    pub icmp_insrcquenchs: Option<i64>,
    pub icmp_intimeexcds: Option<i64>,
    pub icmp_intimestampreps: Option<i64>,
    pub icmp_intimestamps: Option<i64>,
    pub icmp_outaddrmaskreps: Option<i64>,
    pub icmp_outaddrmasks: Option<i64>,
    pub icmp_outdestunreachs: Option<i64>,
    pub icmp_outechoreps: Option<i64>,
    pub icmp_outechos: Option<i64>,
    pub icmp_outerrors: Option<i64>,
    pub icmp_outmsgs: Option<i64>,
    pub icmp_outparmprobs: Option<i64>,
    pub icmp_outratelimitglobal: Option<i64>,
    pub icmp_outratelimithost: Option<i64>,
    pub icmp_outredirects: Option<i64>,
    pub icmp_outsrcquenchs: Option<i64>,
    pub icmp_outtimeexcds: Option<i64>,
    pub icmp_outtimestampreps: Option<i64>,
    pub icmp_outtimestamps: Option<i64>,
    pub icmpmsg_intype3: Option<i64>,
    pub icmpmsg_intype5: Option<i64>,
    pub icmpmsg_outtype3: Option<i64>,
    pub ip_defaultttl: Option<i64>,
    pub ip_forwarding: Option<i64>,
    pub ip_forwdatagrams: Option<i64>,
    pub ip_fragcreates: Option<i64>,
    pub ip_fragfails: Option<i64>,
    pub ip_fragoks: Option<i64>,
    pub ip_inaddrerrors: Option<i64>,
    pub ip_indelivers: Option<i64>,
    pub ip_indiscards: Option<i64>,
    pub ip_inhdrerrors: Option<i64>,
    pub ip_inreceives: Option<i64>,
    pub ip_inunknownprotos: Option<i64>,
    pub ip_outdiscards: Option<i64>,
    pub ip_outnoroutes: Option<i64>,
    pub ip_outrequests: Option<i64>,
    pub ip_outtransmits: Option<i64>,
    pub ip_reasmfails: Option<i64>,
    pub ip_reasmoks: Option<i64>,
    pub ip_reasmreqds: Option<i64>,
    pub ip_reasmtimeout: Option<i64>,
    pub packets_recv: Option<i64>,
    pub packets_sent: Option<i64>,
    pub speed: Option<i64>,
    pub tcp_activeopens: Option<i64>,
    pub tcp_attemptfails: Option<i64>,
    pub tcp_currestab: Option<i64>,
    pub tcp_estabresets: Option<i64>,
    pub tcp_incsumerrors: Option<i64>,
    pub tcp_inerrs: Option<i64>,
    pub tcp_insegs: Option<i64>,
    pub tcp_maxconn: Option<i64>,
    pub tcp_outrsts: Option<i64>,
    pub tcp_outsegs: Option<i64>,
    pub tcp_passiveopens: Option<i64>,
    pub tcp_retranssegs: Option<i64>,
    pub tcp_rtoalgorithm: Option<i64>,
    pub tcp_rtomax: Option<i64>,
    pub tcp_rtomin: Option<i64>,
    pub udp_ignoredmulti: Option<i64>,
    pub udp_incsumerrors: Option<i64>,
    pub udp_indatagrams: Option<i64>,
    pub udp_inerrors: Option<i64>,
    pub udp_memerrors: Option<i64>,
    pub udp_noports: Option<i64>,
    pub udp_outdatagrams: Option<i64>,
    pub udp_rcvbuferrors: Option<i64>,
    pub udp_sndbuferrors: Option<i64>,
    pub udplite_ignoredmulti: Option<i64>,
    pub udplite_incsumerrors: Option<i64>,
    pub udplite_indatagrams: Option<i64>,
    pub udplite_inerrors: Option<i64>,
    pub udplite_memerrors: Option<i64>,
    pub udplite_noports: Option<i64>,
    pub udplite_outdatagrams: Option<i64>,
    pub udplite_rcvbuferrors: Option<i64>,
    pub udplite_sndbuferrors: Option<i64>,
}

impl ReportFields for NetMetrics {
    fn report_fields(&self, mut metric: ReportMetric) -> ReportMetric {
        if let Some(bytes_recv) = self.bytes_recv {
            metric = metric.with_field("bytes_recv", bytes_recv);
        }
        if let Some(bytes_sent) = self.bytes_sent {
            metric = metric.with_field("bytes_sent", bytes_sent);
        }
        if let Some(drop_in) = self.drop_in {
            metric = metric.with_field("drop_in", drop_in);
        }
        if let Some(drop_out) = self.drop_out {
            metric = metric.with_field("drop_out", drop_out);
        }
        if let Some(err_in) = self.err_in {
            metric = metric.with_field("err_in", err_in);
        }
        if let Some(err_out) = self.err_out {
            metric = metric.with_field("err_out", err_out);
        }
        if let Some(icmp_inaddrmaskreps) = self.icmp_inaddrmaskreps {
            metric = metric.with_field("icmp_inaddrmaskreps", icmp_inaddrmaskreps);
        }
        if let Some(icmp_inaddrmasks) = self.icmp_inaddrmasks {
            metric = metric.with_field("icmp_inaddrmasks", icmp_inaddrmasks);
        }
        if let Some(icmp_incsumerrors) = self.icmp_incsumerrors {
            metric = metric.with_field("icmp_incsumerrors", icmp_incsumerrors);
        }
        if let Some(icmp_indestunreachs) = self.icmp_indestunreachs {
            metric = metric.with_field("icmp_indestunreachs", icmp_indestunreachs);
        }
        if let Some(icmp_inechoreps) = self.icmp_inechoreps {
            metric = metric.with_field("icmp_inechoreps", icmp_inechoreps);
        }
        if let Some(icmp_inechos) = self.icmp_inechos {
            metric = metric.with_field("icmp_inechos", icmp_inechos);
        }
        if let Some(icmp_inerrors) = self.icmp_inerrors {
            metric = metric.with_field("icmp_inerrors", icmp_inerrors);
        }
        if let Some(icmp_inmsgs) = self.icmp_inmsgs {
            metric = metric.with_field("icmp_inmsgs", icmp_inmsgs);
        }
        if let Some(icmp_inparmprobs) = self.icmp_inparmprobs {
            metric = metric.with_field("icmp_inparmprobs", icmp_inparmprobs);
        }
        if let Some(icmp_inredirects) = self.icmp_inredirects {
            metric = metric.with_field("icmp_inredirects", icmp_inredirects);
        }
        if let Some(icmp_insrcquenchs) = self.icmp_insrcquenchs {
            metric = metric.with_field("icmp_insrcquenchs", icmp_insrcquenchs);
        }
        if let Some(icmp_intimeexcds) = self.icmp_intimeexcds {
            metric = metric.with_field("icmp_intimeexcds", icmp_intimeexcds);
        }
        if let Some(icmp_intimestampreps) = self.icmp_intimestampreps {
            metric = metric.with_field("icmp_intimestampreps", icmp_intimestampreps);
        }
        if let Some(icmp_intimestamps) = self.icmp_intimestamps {
            metric = metric.with_field("icmp_intimestamps", icmp_intimestamps);
        }
        if let Some(icmp_outaddrmaskreps) = self.icmp_outaddrmaskreps {
            metric = metric.with_field("icmp_outaddrmaskreps", icmp_outaddrmaskreps);
        }
        if let Some(icmp_outaddrmasks) = self.icmp_outaddrmasks {
            metric = metric.with_field("icmp_outaddrmasks", icmp_outaddrmasks);
        }
        if let Some(icmp_outdestunreachs) = self.icmp_outdestunreachs {
            metric = metric.with_field("icmp_outdestunreachs", icmp_outdestunreachs);
        }
        if let Some(icmp_outechoreps) = self.icmp_outechoreps {
            metric = metric.with_field("icmp_outechoreps", icmp_outechoreps);
        }
        if let Some(icmp_outechos) = self.icmp_outechos {
            metric = metric.with_field("icmp_outechos", icmp_outechos);
        }
        if let Some(icmp_outerrors) = self.icmp_outerrors {
            metric = metric.with_field("icmp_outerrors", icmp_outerrors);
        }
        if let Some(icmp_outmsgs) = self.icmp_outmsgs {
            metric = metric.with_field("icmp_outmsgs", icmp_outmsgs);
        }
        if let Some(icmp_outparmprobs) = self.icmp_outparmprobs {
            metric = metric.with_field("icmp_outparmprobs", icmp_outparmprobs);
        }
        if let Some(icmp_outratelimitglobal) = self.icmp_outratelimitglobal {
            metric = metric.with_field("icmp_outratelimitglobal", icmp_outratelimitglobal);
        }
        if let Some(icmp_outratelimithost) = self.icmp_outratelimithost {
            metric = metric.with_field("icmp_outratelimithost", icmp_outratelimithost);
        }
        if let Some(icmp_outredirects) = self.icmp_outredirects {
            metric = metric.with_field("icmp_outredirects", icmp_outredirects);
        }
        if let Some(icmp_outsrcquenchs) = self.icmp_outsrcquenchs {
            metric = metric.with_field("icmp_outsrcquenchs", icmp_outsrcquenchs);
        }
        if let Some(icmp_outtimeexcds) = self.icmp_outtimeexcds {
            metric = metric.with_field("icmp_outtimeexcds", icmp_outtimeexcds);
        }
        if let Some(icmp_outtimestampreps) = self.icmp_outtimestampreps {
            metric = metric.with_field("icmp_outtimestampreps", icmp_outtimestampreps);
        }
        if let Some(icmp_outtimestamps) = self.icmp_outtimestamps {
            metric = metric.with_field("icmp_outtimestamps", icmp_outtimestamps);
        }
        if let Some(icmpmsg_intype3) = self.icmpmsg_intype3 {
            metric = metric.with_field("icmpmsg_intype3", icmpmsg_intype3);
        }
        if let Some(icmpmsg_intype5) = self.icmpmsg_intype5 {
            metric = metric.with_field("icmpmsg_intype5", icmpmsg_intype5);
        }
        if let Some(icmpmsg_outtype3) = self.icmpmsg_outtype3 {
            metric = metric.with_field("icmpmsg_outtype3", icmpmsg_outtype3);
        }
        if let Some(ip_defaultttl) = self.ip_defaultttl {
            metric = metric.with_field("ip_defaultttl", ip_defaultttl);
        }
        if let Some(ip_forwarding) = self.ip_forwarding {
            metric = metric.with_field("ip_forwarding", ip_forwarding);
        }
        if let Some(ip_forwdatagrams) = self.ip_forwdatagrams {
            metric = metric.with_field("ip_forwdatagrams", ip_forwdatagrams);
        }
        if let Some(ip_fragcreates) = self.ip_fragcreates {
            metric = metric.with_field("ip_fragcreates", ip_fragcreates);
        }
        if let Some(ip_fragfails) = self.ip_fragfails {
            metric = metric.with_field("ip_fragfails", ip_fragfails);
        }
        if let Some(ip_fragoks) = self.ip_fragoks {
            metric = metric.with_field("ip_fragoks", ip_fragoks);
        }
        if let Some(ip_inaddrerrors) = self.ip_inaddrerrors {
            metric = metric.with_field("ip_inaddrerrors", ip_inaddrerrors);
        }
        if let Some(ip_indelivers) = self.ip_indelivers {
            metric = metric.with_field("ip_indelivers", ip_indelivers);
        }
        if let Some(ip_indiscards) = self.ip_indiscards {
            metric = metric.with_field("ip_indiscards", ip_indiscards);
        }
        if let Some(ip_inhdrerrors) = self.ip_inhdrerrors {
            metric = metric.with_field("ip_inhdrerrors", ip_inhdrerrors);
        }
        if let Some(ip_inreceives) = self.ip_inreceives {
            metric = metric.with_field("ip_inreceives", ip_inreceives);
        }
        if let Some(ip_inunknownprotos) = self.ip_inunknownprotos {
            metric = metric.with_field("ip_inunknownprotos", ip_inunknownprotos);
        }
        if let Some(ip_outdiscards) = self.ip_outdiscards {
            metric = metric.with_field("ip_outdiscards", ip_outdiscards);
        }
        if let Some(ip_outnoroutes) = self.ip_outnoroutes {
            metric = metric.with_field("ip_outnoroutes", ip_outnoroutes);
        }
        if let Some(ip_outrequests) = self.ip_outrequests {
            metric = metric.with_field("ip_outrequests", ip_outrequests);
        }
        if let Some(ip_outtransmits) = self.ip_outtransmits {
            metric = metric.with_field("ip_outtransmits", ip_outtransmits);
        }
        if let Some(ip_reasmfails) = self.ip_reasmfails {
            metric = metric.with_field("ip_reasmfails", ip_reasmfails);
        }
        if let Some(ip_reasmoks) = self.ip_reasmoks {
            metric = metric.with_field("ip_reasmoks", ip_reasmoks);
        }
        if let Some(ip_reasmreqds) = self.ip_reasmreqds {
            metric = metric.with_field("ip_reasmreqds", ip_reasmreqds);
        }
        if let Some(ip_reasmtimeout) = self.ip_reasmtimeout {
            metric = metric.with_field("ip_reasmtimeout", ip_reasmtimeout);
        }
        if let Some(packets_recv) = self.packets_recv {
            metric = metric.with_field("packets_recv", packets_recv);
        }
        if let Some(packets_sent) = self.packets_sent {
            metric = metric.with_field("packets_sent", packets_sent);
        }
        if let Some(speed) = self.speed {
            metric = metric.with_field("speed", speed);
        }
        if let Some(tcp_activeopens) = self.tcp_activeopens {
            metric = metric.with_field("tcp_activeopens", tcp_activeopens);
        }
        if let Some(tcp_attemptfails) = self.tcp_attemptfails {
            metric = metric.with_field("tcp_attemptfails", tcp_attemptfails);
        }
        if let Some(tcp_currestab) = self.tcp_currestab {
            metric = metric.with_field("tcp_currestab", tcp_currestab);
        }
        if let Some(tcp_estabresets) = self.tcp_estabresets {
            metric = metric.with_field("tcp_estabresets", tcp_estabresets);
        }
        if let Some(tcp_incsumerrors) = self.tcp_incsumerrors {
            metric = metric.with_field("tcp_incsumerrors", tcp_incsumerrors);
        }
        if let Some(tcp_inerrs) = self.tcp_inerrs {
            metric = metric.with_field("tcp_inerrs", tcp_inerrs);
        }
        if let Some(tcp_insegs) = self.tcp_insegs {
            metric = metric.with_field("tcp_insegs", tcp_insegs);
        }
        if let Some(tcp_maxconn) = self.tcp_maxconn {
            metric = metric.with_field("tcp_maxconn", tcp_maxconn);
        }
        if let Some(tcp_outrsts) = self.tcp_outrsts {
            metric = metric.with_field("tcp_outrsts", tcp_outrsts);
        }
        if let Some(tcp_outsegs) = self.tcp_outsegs {
            metric = metric.with_field("tcp_outsegs", tcp_outsegs);
        }
        if let Some(tcp_passiveopens) = self.tcp_passiveopens {
            metric = metric.with_field("tcp_passiveopens", tcp_passiveopens);
        }
        if let Some(tcp_retranssegs) = self.tcp_retranssegs {
            metric = metric.with_field("tcp_retranssegs", tcp_retranssegs);
        }
        if let Some(tcp_rtoalgorithm) = self.tcp_rtoalgorithm {
            metric = metric.with_field("tcp_rtoalgorithm", tcp_rtoalgorithm);
        }
        if let Some(tcp_rtomax) = self.tcp_rtomax {
            metric = metric.with_field("tcp_rtomax", tcp_rtomax);
        }
        if let Some(tcp_rtomin) = self.tcp_rtomin {
            metric = metric.with_field("tcp_rtomin", tcp_rtomin);
        }
        if let Some(udp_ignoredmulti) = self.udp_ignoredmulti {
            metric = metric.with_field("udp_ignoredmulti", udp_ignoredmulti);
        }
        if let Some(udp_incsumerrors) = self.udp_incsumerrors {
            metric = metric.with_field("udp_incsumerrors", udp_incsumerrors);
        }
        if let Some(udp_indatagrams) = self.udp_indatagrams {
            metric = metric.with_field("udp_indatagrams", udp_indatagrams);
        }
        if let Some(udp_inerrors) = self.udp_inerrors {
            metric = metric.with_field("udp_inerrors", udp_inerrors);
        }
        if let Some(udp_memerrors) = self.udp_memerrors {
            metric = metric.with_field("udp_memerrors", udp_memerrors);
        }
        if let Some(udp_noports) = self.udp_noports {
            metric = metric.with_field("udp_noports", udp_noports);
        }
        if let Some(udp_outdatagrams) = self.udp_outdatagrams {
            metric = metric.with_field("udp_outdatagrams", udp_outdatagrams);
        }
        if let Some(udp_rcvbuferrors) = self.udp_rcvbuferrors {
            metric = metric.with_field("udp_rcvbuferrors", udp_rcvbuferrors);
        }
        if let Some(udp_sndbuferrors) = self.udp_sndbuferrors {
            metric = metric.with_field("udp_sndbuferrors", udp_sndbuferrors);
        }
        if let Some(udplite_ignoredmulti) = self.udplite_ignoredmulti {
            metric = metric.with_field("udplite_ignoredmulti", udplite_ignoredmulti);
        }
        if let Some(udplite_incsumerrors) = self.udplite_incsumerrors {
            metric = metric.with_field("udplite_incsumerrors", udplite_incsumerrors);
        }
        if let Some(udplite_indatagrams) = self.udplite_indatagrams {
            metric = metric.with_field("udplite_indatagrams", udplite_indatagrams);
        }
        if let Some(udplite_inerrors) = self.udplite_inerrors {
            metric = metric.with_field("udplite_inerrors", udplite_inerrors);
        }
        if let Some(udplite_memerrors) = self.udplite_memerrors {
            metric = metric.with_field("udplite_memerrors", udplite_memerrors);
        }
        if let Some(udplite_noports) = self.udplite_noports {
            metric = metric.with_field("udplite_noports", udplite_noports);
        }
        if let Some(udplite_outdatagrams) = self.udplite_outdatagrams {
            metric = metric.with_field("udplite_outdatagrams", udplite_outdatagrams);
        }
        if let Some(udplite_rcvbuferrors) = self.udplite_rcvbuferrors {
            metric = metric.with_field("udplite_rcvbuferrors", udplite_rcvbuferrors);
        }
        if let Some(udplite_sndbuferrors) = self.udplite_sndbuferrors {
            metric = metric.with_field("udplite_sndbuferrors", udplite_sndbuferrors);
        }
        metric
    }
}

/// [`super::HostMetricsName::Netstat`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetstatMetrics {
    pub tcp_close: i64,
    pub tcp_close_wait: i64,
    pub tcp_closing: i64,
    pub tcp_established: i64,
    pub tcp_fin_wait1: i64,
    pub tcp_fin_wait2: i64,
    pub tcp_last_ack: i64,
    pub tcp_listen: i64,
    pub tcp_none: i64,
    pub tcp_syn_recv: i64,
    pub tcp_syn_sent: i64,
    pub tcp_time_wait: i64,
    pub udp_socket: i64,
}

impl ReportFields for NetstatMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("tcp_close", self.tcp_close)
            .with_field("tcp_close_wait", self.tcp_close_wait)
            .with_field("tcp_closing", self.tcp_closing)
            .with_field("tcp_established", self.tcp_established)
            .with_field("tcp_fin_wait1", self.tcp_fin_wait1)
            .with_field("tcp_fin_wait2", self.tcp_fin_wait2)
            .with_field("tcp_last_ack", self.tcp_last_ack)
            .with_field("tcp_listen", self.tcp_listen)
            .with_field("tcp_none", self.tcp_none)
            .with_field("tcp_syn_recv", self.tcp_syn_recv)
            .with_field("tcp_syn_sent", self.tcp_syn_sent)
            .with_field("tcp_time_wait", self.tcp_time_wait)
            .with_field("udp_socket", self.udp_socket)
    }
}

/// [`super::HostMetricsName::Processes`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProcessesMetrics {
    pub blocked: i64,
    pub dead: i64,
    pub idle: i64,
    pub paging: i64,
    pub running: i64,
    pub sleeping: i64,
    pub stopped: i64,
    pub total: i64,
    pub total_threads: i64,
    pub unknown: i64,
    pub zombies: i64,
}

impl ReportFields for ProcessesMetrics {
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_field("blocked", self.blocked)
            .with_field("dead", self.dead)
            .with_field("idle", self.idle)
            .with_field("paging", self.paging)
            .with_field("running", self.running)
            .with_field("sleeping", self.sleeping)
            .with_field("stopped", self.stopped)
            .with_field("total", self.total)
            .with_field("total_threads", self.total_threads)
            .with_field("unknown", self.unknown)
            .with_field("zombies", self.zombies)
    }
}

/// [`super::HostMetricsName::Swap`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMetrics {
    pub free: Option<i64>,
    pub total: Option<i64>,
    pub used: Option<i64>,
    pub used_percent: Option<f64>,
    pub r#in: Option<i64>,
    pub out: Option<i64>,
}

impl ReportFields for SwapMetrics {
    fn report_fields(&self, mut metric: ReportMetric) -> ReportMetric {
        if let Some(free) = self.free {
            metric = metric.with_field("free", free);
        }
        if let Some(total) = self.total {
            metric = metric.with_field("total", total);
        }
        if let Some(used) = self.used {
            metric = metric.with_field("used", used);
        }
        if let Some(used_percent) = self.used_percent {
            metric = metric.with_field("used_percent", used_percent);
        }
        if let Some(r#in) = self.r#in {
            metric = metric.with_field("in", r#in);
        }
        if let Some(out) = self.out {
            metric = metric.with_field("out", out);
        }
        metric
    }
}

/// [`super::HostMetricsName::System`] [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemMetrics {
    pub load1: Option<f64>,
    pub load5: Option<f64>,
    pub load15: Option<f64>,
    pub n_cpus: Option<i64>,
    pub n_unique_users: Option<i64>,
    pub n_users: Option<i64>,
    pub uptime: Option<i64>,
    pub uptime_format: Option<String>,
}

impl ReportFields for SystemMetrics {
    fn report_fields(&self, mut metric: ReportMetric) -> ReportMetric {
        if let Some(load1) = self.load1 {
            metric = metric.with_field("load1", load1);
        }
        if let Some(load5) = self.load5 {
            metric = metric.with_field("load5", load5);
        }
        if let Some(load15) = self.load15 {
            metric = metric.with_field("load15", load15);
        }
        if let Some(n_cpus) = self.n_cpus {
            metric = metric.with_field("n_cpus", n_cpus);
        }
        if let Some(n_unique_users) = self.n_unique_users {
            metric = metric.with_field("n_unique_users", n_unique_users);
        }
        if let Some(n_users) = self.n_users {
            metric = metric.with_field("n_users", n_users);
        }
        if let Some(uptime) = self.uptime {
            metric = metric.with_field("uptime", uptime);
        }
        if let Some(uptime_format) = &self.uptime_format {
            metric = metric.with_field("uptime_format", uptime_format.clone());
        }
        metric
    }
}
