package com.thejebforge.badgeproject.util

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.app.ActivityCompat
import java.util.UUID

@SuppressLint("MissingPermission")
class GATTHelper private constructor(
    private val connectionCallback: (Pair<GATTHelper, Boolean>) -> Unit
) : BluetoothGattCallback() {
    //region Connection to the device
    private lateinit var gattServer: BluetoothGatt

    companion object {
        fun connect(
            context: Context,
            device: BluetoothDevice,
            connectionCallback: (Pair<GATTHelper, Boolean>) -> Unit
        ): Result<GATTHelper> = runCatching {
            GATTHelper(
                connectionCallback
            ).apply {
                if (ActivityCompat.checkSelfPermission(
                        context,
                        Manifest.permission.BLUETOOTH_CONNECT
                    ) != PackageManager.PERMISSION_GRANTED
                ) {
                    throw IllegalStateException("No bluetooth permission")
                }
                gattServer = device.connectGatt(context, false, this)
            }
        }
    }

    //endregion

    //region Command handling
    private sealed class Command {
        data class DiscoverServices(
            val onDiscovered: (success: Boolean) -> Unit
        ) : Command()

        data class ReadCharacteristic(
            val svcUUID: UUID,
            val chrUUID: UUID,
            val onRead: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit
        ) : Command()
    }

    private val commandDeque: ArrayDeque<Command> = ArrayDeque<_>()
    private var currentCommand: Command? = null
    private var waiting: Boolean = false

    private fun appendCommand(command: Command) {
        commandDeque.add(command)
        tryStartExecution()
    }

    private fun getCharacteristic(svcUUID: UUID, chrUUID: UUID): BluetoothGattCharacteristic? =
        gattServer.getService(svcUUID)
            ?.getCharacteristic(chrUUID)

    private fun tryStartExecution() {
        if (currentCommand != null && waiting) return
        if (commandDeque.isEmpty()) return

        waiting = true
        val command = commandDeque.removeFirst()
        currentCommand = command

        when (command) {
            is Command.DiscoverServices -> {
                if (!gattServer.discoverServices()) {
                    command.onDiscovered(false)
                    continueExecution()
                }
            }

            is Command.ReadCharacteristic -> {
                val chr = getCharacteristic(command.svcUUID, command.chrUUID)

                if (!gattServer.readCharacteristic(chr)) {
                    command.onRead(Pair(null, null))
                    continueExecution()
                }
            }

            else -> {
                throw IllegalStateException("Unknown command! $currentCommand")
            }
        }
    }

    private fun continueExecution() {
        waiting = false
        tryStartExecution()
    }
    //endregion

    //region Public API
    fun discoverServices(callback: (Boolean) -> Unit) = this.apply {
        appendCommand(Command.DiscoverServices(callback))
    }

    inner class Service internal constructor(
        private val svcUUID: UUID
    ) {
        fun readCharacteristic(chrUUID: UUID, readCallback: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit) {
            appendCommand(
                Command.ReadCharacteristic(
                    svcUUID,
                    chrUUID,
                    readCallback
                )
            )
        }

        fun readCharacteristic(chrUUID: String, readCallback: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit) =
            readCharacteristic(
                UUID.fromString(chrUUID),
                readCallback
            )
    }

    fun withService(svcUUID: UUID, withCallback: Service.() -> Unit) = this.apply {
        withCallback(Service(svcUUID))
    }

    fun withService(svcUUID: String, withCallback: Service.() -> Unit) = withService(
        UUID.fromString(svcUUID),
        withCallback
    )

    fun disconnect() = gattServer.disconnect()
    //endregion

    //region Overrides
    override fun onConnectionStateChange(
        gatt: BluetoothGatt?,
        status: Int,
        newState: Int
    ) {
        when (newState) {
            BluetoothProfile.STATE_CONNECTED -> {
                connectionCallback(Pair(this, true))
            }

            BluetoothProfile.STATE_DISCONNECTED -> {
                connectionCallback(Pair(this, false))
            }
        }
    }

    override fun onServicesDiscovered(gatt: BluetoothGatt?, status: Int) {
        super.onServicesDiscovered(gatt, status)

        when (currentCommand) {
            is Command.DiscoverServices -> {
                (currentCommand as Command.DiscoverServices)
                    .onDiscovered(status == BluetoothGatt.GATT_SUCCESS)
                continueExecution()
            }

            else -> {}
        }
    }

    override fun onCharacteristicRead(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray,
        status: Int
    ) {
        super.onCharacteristicRead(gatt, characteristic, value, status)

        when (val command = currentCommand) {
            is Command.ReadCharacteristic -> {
                command.onRead(
                    Pair(
                        characteristic,
                        if (status == BluetoothGatt.GATT_SUCCESS) value else null
                    )
                )
                continueExecution()
            }

            else -> {}
        }
    }

    //endregion
}