use lazy_static::lazy_static;
use prometheus::{
    histogram_opts, opts, register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramOpts, HistogramVec,
};
const PROM_NAMESPACE: &str = "sunspec_rs";

macro_rules! app_opts {
    ($a:expr, $b:expr) => {
        opts!($a, $b).namespace(PROM_NAMESPACE)
    };
}
macro_rules! app_histogram_opts {
    ($a:expr, $b:expr, $c:expr) => {
        histogram_opts!($a, $b, $c).namespace(PROM_NAMESPACE)
    };
}

lazy_static! {
    // list o' metrics
    pub static ref MODBUS_GET: HistogramVec = register_histogram_vec!(
        app_histogram_opts!(
            "modbus_get_duration_seconds",
            "histogram of timings for get operations",
            vec![0.0,
                0.005,
                0.05,
                0.1,
                0.2,
                0.4,
                0.8,
                1.0,
                2.0,
                5.0,
                10.0]
        ),
        &["modbus_type"]
    ).unwrap();
        pub static ref MODBUS_SET: HistogramVec = register_histogram_vec!(
        app_histogram_opts!(
            "modbus_set_duration_seconds",
            "histogram of timings for set operations",
            vec![0.0,
                0.005,
                0.05,
                0.1,
                0.2,
                0.4,
                0.8,
                1.0,
                2.0,
                5.0,
                10.0]
        ),
        &["modbus_type"]
    ).unwrap();
}
