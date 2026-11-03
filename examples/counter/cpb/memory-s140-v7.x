/* Adafruit Circuit Playground Bluefruit (nRF52840, 1 MB flash, 256 KB RAM),
 * with the Adafruit nRF52 UF2 bootloader and SoftDevice S140 7.x. Cargo feature
 * `sd-v7` selects this file.
 *
 * Mirrors Adafruit's own nrf52840_s140_v7.ld (Adafruit_nRF52_Arduino):
 *   0x00000000..0x00001000  MBR
 *   0x00001000..0x00027000  SoftDevice S140 7.x
 *   0x00027000..0x000ED000  application (this)
 *   0x000ED000..0x000F4000  user data / filesystem (left alone)
 *   0x000F4000..            UF2 bootloader + settings
 * RAM: the first 0x6000 is left for the SoftDevice, although this app never
 * enables it. A board with S140 6.1.1 uses memory-s140-v6.x instead.
 */
MEMORY
{
  FLASH : ORIGIN = 0x00027000, LENGTH = 0xED000 - 0x27000
  RAM   : ORIGIN = 0x20006000, LENGTH = 0x20040000 - 0x20006000
}
