package app.uuob.photos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Photo
import androidx.compose.runtime.remember
import androidx.lifecycle.lifecycleScope
import app.uuob.kit.AppInfo
import app.uuob.kit.AppTheme
import app.uuob.kit.Colors
import app.uuob.kit.Gate
import app.uuob.kit.Updater
import app.uuob.kit.Uuob

/** Who Photos is, for UUOB and its connect screen. */
val photosApp = AppInfo(
  id = "photos",
  name = "Photos",
  noun = "photos",
  icon = Icons.Rounded.Photo,
  color = Colors.of("orange"),
  version = BuildConfig.VERSION_NAME,
  updateFeed = "https://raw.githubusercontent.com/jachymri/uuob-releases/main/photos.json",
  updateKey = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU5QjY3MTNBQjA5NTI0QzAKUldUQUpKV3dPbkcyV2VzZU9XSWJQT29NWTR3angxNjU1VWJZazJsMi80Zjl6WDJoSWxMK0t3MlUK",
)

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    Uuob.init(applicationContext, photosApp)
    val updater = Updater(applicationContext, lifecycleScope)
    setContent {
      AppTheme(photosApp.color) {
        val model = remember { PhotosModel(applicationContext) }
        Gate(updater) { PhotosScreen(model, updater) }
      }
    }
  }
}
