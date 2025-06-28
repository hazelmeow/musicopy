package zip.meows.musicopy.ui

import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning

@Composable
actual fun QRScanner(onResult: (String) -> Unit) {
    val context = LocalContext.current
    var result by remember { mutableStateOf("") }

    Button(onClick = {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(
                Barcode.FORMAT_QR_CODE,
            )
            .build()

        val scanner = GmsBarcodeScanning.getClient(context, options)

        scanner.startScan()
            .addOnSuccessListener { barcode ->
                result = barcode.rawValue ?: ""

                onResult(result)
                // Task completed successfully
            }
            .addOnCanceledListener {
                // Task canceled
                result = "cancelled"
            }
            .addOnFailureListener { e ->
                // Task failed with an exception
                result = "error: ${e}"
            }
    }) {
        Text("Scan QR code")
        Text("Result: ${result}")
    }
}