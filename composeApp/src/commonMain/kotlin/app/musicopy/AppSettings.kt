@file:OptIn(ExperimentalSettingsApi::class)

package app.musicopy

import com.russhwolf.settings.ExperimentalSettingsApi
import com.russhwolf.settings.ObservableSettings
import com.russhwolf.settings.coroutines.getStringOrNullFlow
import com.russhwolf.settings.observable.makeObservable
import kotlinx.coroutines.flow.Flow
import com.russhwolf.settings.Settings

const val DOWNLOAD_DIRECTORY_KEY = "downloadDirectory"

object AppSettings {
    private val settings: ObservableSettings = Settings().makeObservable()

    var downloadDirectory: String?
        get() = settings.getStringOrNull(DOWNLOAD_DIRECTORY_KEY)
        set(value) {
            value?.let {
                settings.putString(DOWNLOAD_DIRECTORY_KEY, value)
            } ?: {
                settings.remove(DOWNLOAD_DIRECTORY_KEY)
            }
        }

    val downloadDirectoryFlow: Flow<String?>
        get() = settings.getStringOrNullFlow(DOWNLOAD_DIRECTORY_KEY)
}
