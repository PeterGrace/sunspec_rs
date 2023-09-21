
# 2023-09-21 (v0.5.2)

## Added

- Added ability to do integration tests without requiring an actual sunspec device - data is mocked as a Vec<u16> buffer.

## Changed

- ModelData.model was private, now made public to facilitate usage in test harness

# 2023-09-20 (v0.5.1)

## Added

- `strict_symbols` to `SunSpecConnection` struct, allowing `sunspec_rs` to synthesize names for enums/bitfields if the model is inaccurate

# 2023-09-20 (v0.5.0)

## Changed

- vendor model files are now stored in models/{vendor-name-lowercase}/smdx_?????.xml.  Vendor name is found by checking the Mn field of the common model (1).
