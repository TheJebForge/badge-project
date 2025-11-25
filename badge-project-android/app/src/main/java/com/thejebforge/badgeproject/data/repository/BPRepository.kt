package com.thejebforge.badgeproject.data.repository

import com.thejebforge.badgeproject.data.dao.PreviousDeviceDao
import com.thejebforge.badgeproject.data.entity.PreviousDevice
import com.thejebforge.badgeproject.util.Response
import javax.inject.Inject

class BPRepository @Inject constructor(
    private val dao: PreviousDeviceDao
) {
    suspend fun getPreviousDevices(): Response<List<PreviousDevice>> = try {
        Response.Success(dao.getAll())
    } catch (e: Exception) {
        Response.Error(e)
    }

    suspend fun getPreviousDevice(mac: String): Response<PreviousDevice?> = try {
        Response.Success(dao.findByMac(mac))
    } catch (e: Exception) {
        Response.Error(e)
    }

    suspend fun addPreviousDevice(device: PreviousDevice) {
        dao.insert(device)
    }

    suspend fun deletePreviousDevice(device: PreviousDevice) {
        dao.delete(device)
    }
}