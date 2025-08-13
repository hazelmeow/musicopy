package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.LoadingButton
import app.musicopy.ui.components.TopBar

@Composable
fun ConnectManuallyScreen(
    onShowNodeStatus: () -> Unit,

    isConnecting: Boolean,
    onSubmit: (String) -> Unit,
    onCancel: () -> Unit,
) {
    var value by remember { mutableStateOf("") }

    val isEmpty = value.isEmpty()
    val isValid = value.length == 64
    val isError = !isEmpty && !isValid
    val supportingText = if (isError) {
        @Composable {
            Text("Invalid node ID.")
        }
    } else {
//        @Composable {
//            Text("")
//        }
        null
    }

    Scaffold(
        topBar = {
            TopBar(
                title = "Connect manually",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                label = {
                    Text("Node ID")
                },
                maxLines = 1,
                modifier = Modifier.fillMaxWidth(),
                isError = isError,
                supportingText = supportingText
            )

            Info {
                Text("lorem")
            }

            Info {
                Text("desktop install link >")
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                LoadingButton(
                    label = "Connect",
                    onClick = { onSubmit(value) },
                    enabled = isValid,
                    loading = isConnecting,
                )
            }
        }
    }
}

@Composable
fun ConnectManuallyScreenSandbox() {
    var isConnecting by remember { mutableStateOf(false) }

    ConnectManuallyScreen(
        isConnecting = isConnecting,
        onSubmit = { isConnecting = true },
        onCancel = { isConnecting = false },
        onShowNodeStatus = {}
    )
}
