package zip.meows.musicopy

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import uniffi.musicopy.Core
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.EventHandler
import uniffi.musicopy.Model

class CoreViewModel : ViewModel(), EventHandler {
    private val _instance = Core(
        this, CoreOptions(
            initLogging = true,
            inMemory = false,
        )
    )
    val instance: Core
        get() = _instance

    private val _state = MutableStateFlow<Model?>(null)
    val state: StateFlow<Model?> = _state

    override fun onUpdate(model: Model) {
        _state.value = model
    }
}
