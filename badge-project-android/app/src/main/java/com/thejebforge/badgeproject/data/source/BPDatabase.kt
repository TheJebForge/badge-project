package com.thejebforge.badgeproject.data.source

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import com.thejebforge.badgeproject.data.dao.PreviousDeviceDao
import com.thejebforge.badgeproject.data.entity.PreviousDevice
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Database(entities = [PreviousDevice::class], version = 1)
abstract class BPDatabase : RoomDatabase() {
    abstract fun previousDeviceDao(): PreviousDeviceDao
}

@Module
@InstallIn(SingletonComponent::class)
object BPDatabaseModule {
    @Provides
    @Singleton
    fun provideBPDatabase(
        @ApplicationContext context: Context
    ): BPDatabase {
        return Room.databaseBuilder(
            context,
            BPDatabase::class.java,
            "bp"
        ).build()
    }

    @Provides
    fun providePreviousDeviceDao(
        database: BPDatabase
    ): PreviousDeviceDao = database.previousDeviceDao()
}