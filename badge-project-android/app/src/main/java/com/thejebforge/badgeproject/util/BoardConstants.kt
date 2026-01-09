package com.thejebforge.badgeproject.util

object BoardConstants {
    const val CHARACTER_SVC = "c4aa52a4-467e-413f-9559-419eb1a367a7"
    const val CURRENT_MODE_CHR = "00000001-467e-413f-9559-419eb1a367a7"
    const val CHARACTER_NAME_CHR = "00000002-467e-413f-9559-419eb1a367a7"
    const val CHARACTER_SPECIES_CHR = "00000003-467e-413f-9559-419eb1a367a7"
    const val ACTION_COUNT_CHR = "00000004-467e-413f-9559-419eb1a367a7"
    const val CHARACTER_COUNT_CHR = "00000005-467e-413f-9559-419eb1a367a7"
    const val COMMAND_CHR = "00000010-467e-413f-9559-419eb1a367a7"
    const val RESPONSE_CHR = "00000011-467e-413f-9559-419eb1a367a7"

    const val SCREEN_SVC = "230521b4-d8c4-4e35-9b91-6327de387d77"
    const val BACKLIGHT_CHR = "00000001-d8c4-4e35-9b91-6327de387d77"

    const val COMMAND_MAGIC = 242.toByte()

    const val GET_ACTION_OP = 0.toByte()
    const val GET_ACTION_DISPLAY_NAME_OP = 1.toByte()
    const val INVOKE_ACTION_OP = 2.toByte()
    const val GET_CHARACTER_NAME_OP = 3.toByte()
    const val SWITCH_CHARACTER_OP = 4.toByte()

    const val COMMAND_PAYLOAD_SIZE = 202
}