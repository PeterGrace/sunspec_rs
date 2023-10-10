
# 2023-10-10 (v0.6.1)

## Fixed

- Adjusted code to remove compiler warning about variables never being read
- Also adjusted same code to remove unnecessary mutability

# 2023-10-03 (v0.6.0)

## Added

- Prometheus metrics for modbus get/set commands

- `get_symbols_for_point()` which will return the symbols for a given point.

- get_point supports repeating blocks.

## Changed

- Adjusted hardcoded 2-address-offset into a const, ADDR_OFFSET.  Explained its significance in the code comments.

- Downgraded `error!` to `debug!` in situations where we're logging an error and then sending that same text upstream in a `Result<T,E>`
- changed variable names to avoid rustc errors about unused values (they're part of the testing apparatus and aren't used anyway)

- set accessibility for symbol-related data to full pub

## Fixed

- Scale-factor mathematical logic was just... wrong.  Fixed it.

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
