use influxdb::InfluxDbWriteable;

/// An InfluxDb measurement.
pub trait InfluxDbMeasurement: InfluxDbWriteable + Send + Sync {
    /// The measurement name.
    const NAME: &'static str;
}
