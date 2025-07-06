package zip.meows.musicopy.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.composeunstyled.ProgressIndicator

@Composable
fun SectionCard(
    title: String,
    body: @Composable () -> Unit,
    onCancel: (() -> Unit)? = null,
    onAction: (() -> Unit)? = null,
    actionLabel: String = "Submit",
    actionEnabled: Boolean = true,
    actionLoading: Boolean? = null,
) {
    val hasAction = onCancel !== null || onAction !== null

    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(title, style = MaterialTheme.typography.titleLarge)

            body()

            if (hasAction) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, alignment = Alignment.End)
                ) {
                    onCancel?.let {
                        OutlinedButton(
                            onClick = onCancel,
                        ) {
                            Text("Cancel")
                        }
                    }
                    onAction?.let {
                        Button(
                            onClick = onAction,
                            contentPadding = if (actionLoading != null) {
                                PaddingValues(horizontal = 12.dp)
                            } else {
                                ButtonDefaults.ContentPadding
                            },
                            enabled = actionEnabled && actionLoading != true
                        ) {
                            Row(
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                when (actionLoading) {
                                    true -> CircularProgressIndicator(
                                        modifier = Modifier.size(16.dp),
                                    )

                                    false -> Box(modifier = Modifier.size(16.dp))

                                    null -> Unit
                                }
                                Text(actionLabel)
                                when (actionLoading) {
                                    true, false -> Box(modifier = Modifier.size(16.dp))

                                    null -> Unit
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
