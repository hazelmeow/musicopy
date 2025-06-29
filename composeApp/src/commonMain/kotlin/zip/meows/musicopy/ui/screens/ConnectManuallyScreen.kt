package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import zip.meows.musicopy.ui.components.SectionCard

@Composable
fun ConnectManuallyScreen(
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
        @Composable {
            Text("")
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize(),
        verticalArrangement = Arrangement.Center,
    ) {
        SectionCard(
            title = "Connect manually",
            body = {
                Text("Enter the node id etc etc etc.")

                Text("TODO: Desktop install link")

                Column(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
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
                }
            },
            onCancel = onCancel,
            onAction = { onSubmit(value) },
            actionLabel = "Connect",
            actionEnabled = isValid
        )
    }
}
