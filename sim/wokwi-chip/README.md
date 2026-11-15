# QMC5883P Wokwi custom chip

Wraps the [`qmc5883p-sim`](../../qmc5883p-sim) behavioral model into a
[Wokwi custom chip](https://docs.wokwi.com/chips-api/getting-started).

```sh
./build.sh   # produces qmc5883p.chip.wasm next to qmc5883p.chip.json
```

Reference it from a project's `wokwi.toml`:

```toml
[[chip]]
name = 'qmc5883p'
binary = '../../sim/wokwi-chip/qmc5883p.chip.wasm'
```

and add a `chip-qmc5883p` part to `diagram.json`.

| Pin   | Function                  |
|-------|---------------------------|
| `SDA` | I²C data (address `0x2C`) |
| `SCL` | I²C clock                 |

The QMC5883P has no interrupt pin; poll `DRDY` in the status register.

Controls `fieldX`, `fieldY`, `fieldZ` (microtesla) are sampled every millisecond, which is
also the model's time step. Drive them from the Wokwi UI or with `set-control` in
automation scenarios. The sliders cover ±200 µT; larger fields can be set as attributes.
