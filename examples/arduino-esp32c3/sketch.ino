// QMC5883P magnetometer demo for the Wokwi web editor (Arduino, ESP32-C3).
//
// Talks to the QMC5883P Wokwi chip over raw I2C and prints field and heading in the same
// format as the Rust example in ../esp32c3. Wiring: SDA = GPIO4, SCL = GPIO5. The
// QMC5883P has no interrupt pin, so the sketch polls the data-ready flag.

#include <Wire.h>

constexpr uint8_t QMC5883P_ADDRESS = 0x2C;
constexpr uint8_t QMC5883P_CHIP_ID = 0x80;

constexpr uint8_t REG_CHIP_ID = 0x00;
constexpr uint8_t REG_XOUT_L = 0x01;
constexpr uint8_t REG_STATUS = 0x09;
constexpr uint8_t REG_CONTROL_1 = 0x0A;
constexpr uint8_t REG_CONTROL_2 = 0x0B;
constexpr uint8_t REG_AXIS_SIGN = 0x29;

constexpr uint8_t AXIS_SIGN_DEFAULT = 0x06;
constexpr uint8_t STATUS_DRDY = 0x01;

// Control2: SOFT_RST, SELF_TEST, and RNG = 8 G with set/reset on.
constexpr uint8_t CONTROL_2_SOFT_RST = 0x80;
constexpr uint8_t CONTROL_2_SELF_TEST = 0x40;
constexpr uint8_t CONTROL_2_RANGE_8G = 0x08;

// Control1: continuous mode for the self-test; normal mode at 50 Hz, OSR1 = 8, OSR2 = 1.
constexpr uint8_t CONTROL_1_CONTINUOUS = 0x03;
constexpr uint8_t CONTROL_1_NORMAL_50HZ = 0x05;
constexpr uint8_t CONTROL_1_SUSPEND_50HZ = 0x04;

// ±8 G full scale.
constexpr float LSB_PER_GAUSS = 3750.0f;

constexpr int PIN_SDA = 4;
constexpr int PIN_SCL = 5;

bool readRegisters(uint8_t reg, uint8_t *buffer, size_t len) {
  Wire.beginTransmission(QMC5883P_ADDRESS);
  Wire.write(reg);
  if (Wire.endTransmission(false) != 0) {
    return false;
  }

  if (Wire.requestFrom(int(QMC5883P_ADDRESS), int(len)) != int(len)) {
    return false;
  }

  for (size_t i = 0; i < len; i++) {
    buffer[i] = Wire.read();
  }

  return true;
}

void writeRegister(uint8_t reg, uint8_t value) {
  Wire.beginTransmission(QMC5883P_ADDRESS);
  Wire.write(reg);
  Wire.write(value);
  Wire.endTransmission();
}

bool dataReady() {
  uint8_t status = 0;
  return readRegisters(REG_STATUS, &status, 1) && (status & STATUS_DRDY);
}

// X, Y, Z as little-endian 16-bit raw counts.
bool readField(int16_t raw[3]) {
  uint8_t data[6];
  if (!readRegisters(REG_XOUT_L, data, sizeof(data))) {
    return false;
  }

  for (int i = 0; i < 3; i++) {
    raw[i] = int16_t(uint16_t(data[2 * i + 1]) << 8 | data[2 * i]);
  }

  return true;
}

void halt() {
  while (true) {
    delay(1000);
  }
}

void setup() {
  Serial.begin(115200);

  Wire.begin(PIN_SDA, PIN_SCL);
  Wire.setClock(400000);

  uint8_t chipId = 0;
  if (!readRegisters(REG_CHIP_ID, &chipId, 1) || chipId != QMC5883P_CHIP_ID) {
    Serial.printf("QMC5883P not found (chip ID 0x%02X)\n", chipId);
    halt();
  }

  Serial.printf("QMC5883P ok chip_id=0x%02X\n", chipId);

  // Soft reset, then set the axis signs.
  writeRegister(REG_CONTROL_2, CONTROL_2_SOFT_RST);
  delayMicroseconds(250);
  writeRegister(REG_AXIS_SIGN, AXIS_SIGN_DEFAULT);

  // Self-test: take a sample in continuous mode, then one with the self-test field on.
  writeRegister(REG_CONTROL_1, 0x00);
  writeRegister(REG_CONTROL_2, 0x00);
  writeRegister(REG_CONTROL_1, CONTROL_1_CONTINUOUS);
  bool ready = false;
  for (int i = 0; i < 100 && !(ready = dataReady()); i++) {
    delay(1);
  }

  int16_t before[3], with[3];
  if (!ready || !readField(before)) {
    Serial.println("self-test failed");
    halt();
  }

  writeRegister(REG_CONTROL_2, CONTROL_2_SELF_TEST);
  delay(5);
  if (!readField(with)) {
    Serial.println("self-test failed");
    halt();
  }

  Serial.printf("self-test dx=%d dy=%d dz=%d mG\n", before[0] - with[0], with[1] - before[1],
                with[2] - before[2]);

  // Reading STATUS clears DRDY, so the self-test sample is not printed.
  dataReady();

  // Normal mode at 50 Hz, OSR1 = 8, ±8 G. Change the range only while suspended.
  writeRegister(REG_CONTROL_1, CONTROL_1_SUSPEND_50HZ);
  writeRegister(REG_CONTROL_2, CONTROL_2_RANGE_8G);
  writeRegister(REG_CONTROL_1, CONTROL_1_NORMAL_50HZ);
  Serial.println("running odr=50Hz range=8G");
}

void loop() {
  static float last[3];
  static bool haveLast = false;

  int16_t raw[3];
  if (dataReady() && readField(raw)) {
    float field[3];
    for (int i = 0; i < 3; i++) {
      field[i] = raw[i] * 100.0f / LSB_PER_GAUSS;
    }

    if (!haveLast || memcmp(field, last, sizeof(field)) != 0) {
      // Heading in degrees (0–360), with X pointing north and the sensor held flat.
      float heading = atan2f(field[1], field[0]) * 180.0f / PI;
      if (heading < 0) {
        heading += 360.0f;
      }

      Serial.printf("mag x=%.1f y=%.1f z=%.1f uT heading=%.0f\n", field[0], field[1], field[2],
                    heading);
      memcpy(last, field, sizeof(field));
      haveLast = true;
    }
  }

  delay(5);
}
