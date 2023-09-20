
# 2023-09-20 (v0.5.1)

## Added

- `strict_symbols` to `SunSpecConnection` struct, allowing `sunspec_rs` to synthesize names for enums/bitfields if the model is inaccurate

# 2023-09-20 (v0.5.0)

## Changed

- vendor model files are now stored in models/{vendor-name-lowercase}/smdx_?????.xml.  Vendor name is found by checking the Mn field of the common model (1).
