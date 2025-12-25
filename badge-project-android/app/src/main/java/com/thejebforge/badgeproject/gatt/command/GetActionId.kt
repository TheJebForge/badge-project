package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.util.PayloadCreator

class GetActionId(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    handler: Handler,
    private val index: UShort,
    private val onDone: (ActionResponse<String>) -> Unit
) : ActionCommand(gattServer, commandHandler, handler) {
    override fun payload(): ByteArray = PayloadCreator.getAction(index)

    override fun received(data: ByteArray) {
        onDone(String(data).trim(Char(0)).asSuccess())
    }

    override fun failed(reason: ActionResponse<Nothing>) {
        onDone(reason)
    }
}

fun GATTHelper.getActionId(index: Int, callback: (ActionResponse<String>) -> Unit) = this.apply {
    commandHandler.appendCommand(GetActionId(
        gattServer,
        commandHandler,
        handler,
        index.toUShort(),
        callback
    ))
}