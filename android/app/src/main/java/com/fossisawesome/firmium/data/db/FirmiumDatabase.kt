package com.fossisawesome.firmium.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(
    entities = [PlayEntity::class, PodcastChannelEntity::class, PodcastEpisodeEntity::class],
    version = 2,
    exportSchema = false,
)
abstract class FirmiumDatabase : RoomDatabase() {
    abstract fun playDao(): PlayDao
    abstract fun podcastDao(): PodcastDao

    companion object {
        @Volatile private var instance: FirmiumDatabase? = null

        fun get(context: Context): FirmiumDatabase =
            instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    FirmiumDatabase::class.java,
                    "firmium_play_history.db",
                )
                    // No Migration objects exist yet for this DB; destructive fallback
                    // just drops/recreates on schema bump (acceptable for local-only
                    // play history / podcast cache, no server-synced data lost).
                    .fallbackToDestructiveMigration(true)
                    .build().also { instance = it }
            }
    }
}
