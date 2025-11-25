package com.thejebforge.badgeproject.util

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.util.Log
import androidx.annotation.RequiresPermission
import androidx.core.app.ActivityCompat
import androidx.core.content.getSystemService
import com.thejebforge.badgeproject.data.intermediate.Device
import dagger.hilt.android.qualifiers.ActivityContext
import javax.inject.Inject

class BluetoothHelper @Inject constructor(
    @param:ActivityContext private val context: Context
) {
    private var bluetoothManager: BluetoothManager? = null
    private val activity: Activity = context.getActivity()!!

    sealed class DeviceScanResponse {
        data object EnablingBluetooth : DeviceScanResponse()
        data object ScanStarted : DeviceScanResponse()
        data class FoundDevices(val devices: List<Device>) : DeviceScanResponse()
        data object ScanStopped : DeviceScanResponse()
        data object AlreadyScanning : DeviceScanResponse()
    }

    private var currentlyScanning: Boolean = false
    private val handler: Handler = Handler(Looper.getMainLooper())

    @SuppressLint("MissingPermission")
    private inner class ScanDeviceCallback(
        private val callback: (DeviceScanResponse) -> Unit
    ) : ScanCallback() {
        override fun onBatchScanResults(results: List<ScanResult?>?) {
            super.onBatchScanResults(results)
            if (results == null) return

            val devices = results.stream()
                .flatMap { device -> if (device != null) {
                    listOf(Device(device.device.name, device.device.address)).stream()
                } else {
                    listOf<Device>().stream()
                }}
                .toList()

            callback(DeviceScanResponse.FoundDevices(devices))
        }

        override fun onScanFailed(errorCode: Int) {
            super.onScanFailed(errorCode)
            currentlyScanning = false
            callback(DeviceScanResponse.ScanStopped)
        }

        override fun onScanResult(callbackType: Int, result: ScanResult?) {
            super.onScanResult(callbackType, result)
            if (result == null) return
            callback(DeviceScanResponse.FoundDevices(
                listOf(Device(result.device.name, result.device.address))
            ))
        }
    }

    @RequiresPermission(allOf = [Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT])
    fun scanForDevices(callback: (DeviceScanResponse) -> Unit) {
        if (bluetoothManager == null)
            bluetoothManager = context.getSystemService<BluetoothManager>()
        val bluetooth = bluetoothManager!!

        val adapter = bluetooth.adapter ?: throw IllegalStateException("The device has no bluetooth!")

        if (!adapter.isEnabled) {
            ActivityCompat.startActivityForResult(
                activity,
                Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE),
                1,
                null
            )
            callback(DeviceScanResponse.EnablingBluetooth)
            return
        }

        val scanner = adapter.bluetoothLeScanner ?: throw IllegalStateException("No BLE scanner")
        val scanCallback = ScanDeviceCallback(callback)


        if (!currentlyScanning) {
            handler.postDelayed({
                currentlyScanning = false
                scanner.stopScan(scanCallback)
                callback(DeviceScanResponse.ScanStopped)
            }, 10000)
            currentlyScanning = true
            scanner.startScan(
                listOf(
                    ScanFilter.Builder()
                        .setServiceUuid(ParcelUuid.fromString("c4aa52a4-467e-413f-9559-419eb1a367a7"))
                        .build()
                ),
                ScanSettings.Builder().build(),
                scanCallback
            )
            callback(DeviceScanResponse.ScanStarted)
        } else {
            callback(DeviceScanResponse.AlreadyScanning)
        }
    }
}