package com.fossisawesome.firmium.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(entities = [PlayEntity::class], version = 1, exportSchema = false)
abstract class FirmiumDatabase : RoomDatabase() {
    abstract fun playDao(): PlayDao

    companion object {
        @Volatile private var instance: FirmiumDatabase? = null

        fun get(context: Context): FirmiumDatabase =
            instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    FirmiumDatabase::class.java,
                    "firmium_play_history.db",
                ).build().also { instance = it }
            }
    }
}
