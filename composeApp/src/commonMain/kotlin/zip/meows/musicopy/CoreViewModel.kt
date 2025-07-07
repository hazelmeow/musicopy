package zip.meows.musicopy

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.CreationExtras
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import uniffi.musicopy.Core
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.EventHandler
import uniffi.musicopy.Model

class CoreViewModel(private val platformContext: PlatformContext) : ViewModel(), EventHandler {
    companion object {
        val PLATFORM_CONTEXT_KEY = object : CreationExtras.Key<PlatformContext> {}

        val Factory: ViewModelProvider.Factory = viewModelFactory {
            initializer {
                val platformContext = this[PLATFORM_CONTEXT_KEY] as PlatformContext
                CoreViewModel(platformContext = platformContext)
            }
        }
    }

    private val _instance = Core(
        eventHandler = this,
        options = CoreProvider.getOptions()
    )
    val instance: Core
        get() = _instance

    private val _state = MutableStateFlow<Model?>(null)
    val state: StateFlow<Model?> = _state

    override fun onUpdate(model: Model) {
        _state.value = model
    }
}
