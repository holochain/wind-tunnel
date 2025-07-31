use serde::{Deserialize, Serialize};
use wind_tunnel_instruments::prelude::ReportMetric;

/// Macro to conditionally add fields to a metric based on their presence.
macro_rules! with_optional_fields {
    ($self:ident, $metric:expr, $($field:ident),+) => {
        {
            let mut m = $metric;
            $(
                if let Some(value) = $self.$field {
                    m = m.with_field(stringify!($field), value);
                }
            )+
            m
        }
    };
}

/// Trait for reporting fields of host metrics.
pub trait ReportFields {
    /// Returns the associated [`ReportMetric`] for this host metrics fields.
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric;
}

/// Tags for [`super::HostMetrics`] fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
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
    Mem(Box<MemMetrics>),
    /// [`super::HostMetricsName::Netstat`] [`super::HostMetrics`].
    Netstat(NetstatMetrics),
    /// [`super::HostMetricsName::Net`] [`super::HostMetrics`].
    Net(Box<NetMetrics>),
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
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        with_optional_fields!(
            self,
            metric,
            bytes_recv,
            bytes_sent,
            drop_in,
            drop_out,
            err_in,
            err_out,
            icmp_inaddrmaskreps,
            icmp_inaddrmasks,
            icmp_incsumerrors,
            icmp_indestunreachs,
            icmp_inechoreps,
            icmp_inechos,
            icmp_inerrors,
            icmp_inmsgs,
            icmp_inparmprobs,
            icmp_inredirects,
            icmp_insrcquenchs,
            icmp_intimeexcds,
            icmp_intimestampreps,
            icmp_intimestamps,
            icmp_outaddrmaskreps,
            icmp_outaddrmasks,
            icmp_outdestunreachs,
            icmp_outechoreps,
            icmp_outechos,
            icmp_outerrors,
            icmp_outmsgs,
            icmp_outparmprobs,
            icmp_outratelimitglobal,
            icmp_outratelimithost,
            icmp_outredirects,
            icmp_outsrcquenchs,
            icmp_outtimeexcds,
            icmp_outtimestampreps,
            icmp_outtimestamps,
            icmpmsg_intype3,
            icmpmsg_intype5,
            icmpmsg_outtype3,
            ip_defaultttl,
            ip_forwarding,
            ip_forwdatagrams,
            ip_fragcreates,
            ip_fragfails,
            ip_fragoks,
            ip_inaddrerrors,
            ip_indelivers,
            ip_indiscards,
            ip_inhdrerrors,
            ip_inreceives,
            ip_inunknownprotos,
            ip_outdiscards,
            ip_outnoroutes,
            ip_outrequests,
            ip_outtransmits,
            ip_reasmfails,
            ip_reasmoks,
            ip_reasmreqds,
            ip_reasmtimeout,
            packets_recv,
            packets_sent,
            speed,
            tcp_activeopens,
            tcp_attemptfails,
            tcp_currestab,
            tcp_estabresets,
            tcp_incsumerrors,
            tcp_inerrs,
            tcp_insegs,
            tcp_maxconn,
            tcp_outrsts,
            tcp_outsegs,
            tcp_passiveopens,
            tcp_retranssegs,
            tcp_rtoalgorithm,
            tcp_rtomax,
            tcp_rtomin,
            udp_ignoredmulti,
            udp_incsumerrors,
            udp_indatagrams,
            udp_inerrors,
            udp_memerrors,
            udp_noports,
            udp_outdatagrams,
            udp_rcvbuferrors,
            udp_sndbuferrors,
            udplite_ignoredmulti,
            udplite_incsumerrors,
            udplite_indatagrams,
            udplite_inerrors,
            udplite_memerrors,
            udplite_noports,
            udplite_outdatagrams,
            udplite_rcvbuferrors,
            udplite_sndbuferrors
        )
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
    fn report_fields(&self, metric: ReportMetric) -> ReportMetric {
        with_optional_fields!(self, metric, free, total, used, used_percent, r#in, out)
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
        metric = with_optional_fields!(
            self,
            metric,
            load1,
            load5,
            load15,
            n_cpus,
            n_unique_users,
            n_users,
            uptime
        );
        if let Some(uptime_format) = &self.uptime_format {
            metric = metric.with_field("uptime_format", uptime_format.clone());
        }
        metric
    }
}
