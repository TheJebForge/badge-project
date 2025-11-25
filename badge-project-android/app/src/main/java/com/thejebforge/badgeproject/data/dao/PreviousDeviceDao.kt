package com.thejebforge.badgeproject.data.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import com.thejebforge.badgeproject.data.entity.PreviousDevice
import kotlinx.coroutines.flow.Flow

@Dao
interface PreviousDeviceDao {
    @Query("SELECT * FROM previousdevice")
    suspend fun getAll(): List<PreviousDevice>

    @Query("SELECT * FROM previousdevice where macAddress = :mac")
    suspend fun findByMac(mac: String): PreviousDevice?

    @Insert
    suspend fun insert(device: PreviousDevice)

    @Delete
    suspend fun delete(device: PreviousDevice)
}