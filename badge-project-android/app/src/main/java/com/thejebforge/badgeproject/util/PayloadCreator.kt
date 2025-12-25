package com.thejebforge.badgeproject.util

import java.nio.ByteBuffer
import java.nio.ByteOrder

object PayloadCreator {
    private fun getNewBuffer(): ByteBuffer = ByteBuffer.allocate(BoardConstants.COMMAND_PAYLOAD_SIZE)

    private fun ByteBuffer.putHeader(op: Byte) = this.apply {
        order(ByteOrder.LITTLE_ENDIAN)
        put(BoardConstants.COMMAND_MAGIC)
        put(op)
    }

    fun getAction(index: UShort): ByteArray {
        val buf = getNewBuffer().putHeader(BoardConstants.GET_ACTION_OP)
        buf.putShort(index.toShort())
        return buf.array()
    }

    fun getActionDisplayName(id: String): ByteArray {
        val buf = getNewBuffer().putHeader(BoardConstants.GET_ACTION_DISPLAY_NAME_OP)
        buf.put(id.toByteArray())
        return buf.array()
    }
}

