#include <HardwareSerial.h>

HardwareSerial LoRa(2);
void setup() {
  Serial.begin(115200);
  LoRa.begin(115200, SERIAL_8N1, 22, 21);
}

void loop() {
  delay(1000);
  String readings = "";
  for (int i = 0; i < 11; i++) {
    readings += String(random(1, 256));
    if (i < 10) {
      readings += " ";
    }
  }
  LoRa.println("radio tx " + into_hex(readings) + " 1");
}

String into_hex(String str) {
  String res = "";
  int len = str.length();
  for (int i = 0; i < len; i++) {
    byte c = (byte)str.charAt(i);
    if (c < 16) {
      res += "0";
    }
    res += String(c, HEX);
  }
  return res;
}