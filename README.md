## Description

Alfred is an opinionated Kotlin Multiplatform Mobile CLI for creating common and platform-specific components.

## Installation

- Download binary for OS for latest stable release.
- Include in your PATH.

## Setup

- Setup `alfred.yaml` (see [configuration](#configuration)) file at root of project (wherever you'd invoke the CLI).
- Invoke `alfred <command>`

    - `alfred --help` to see list of available commands.

## Configuration

```yaml
android:
  module: "androidApp" # android platform module
  base_package_dir: "tmp" # base package directory (separated by '/') of android module, can be empty to always provide fully qualified package.

common:
  module: "common" # name of common module defaults to "common"
  base_package_dir: "tmp" #  base package directory (separated by '/') of common source set, can be empty to always provide fully qualified package.
  common_source_set: "commonMain" # name of common main source set defaults to "commonMain"
  android_source_set: "androidMain" # name of common android source set defaults to "androidMain"
  ios_source_set: "iosMain" # name of common ios source set defaults to "iosMain"

use_koin: true # indicates where to include Koin definitions for base classes if not present.
```

## Examples

#### Create ViewModel

- `alfred create viewmodel`
- fill out prompts.
- resulting ViewModel:

```kotlin
package tmp.com.example.foobar

private typealias SuccessState = UiState.Success<FoobarViewModel.Success, FoobarViewModel.Error>
private typealias ErrorState = UiState.Error<FoobarViewModel.Success, FoobarViewModel.Error>

class FoobarViewModel : ViewStateViewModel<FoobarViewModel.ViewState>(ViewState()) {

    data class ViewState(
        val ui: UiState<Success, Error> = UiState.Loading()
    )

    object Success

    object Error

    private val initializeJob = AtomicJob()

    fun initialize() {
        initializeJob.value = launch {
            // TODO: replace me
        }
    }

    override fun cleanup() {
        super.cleanup()
        initializeJob.cleanup()
    }

}
```

#### Create Jetpack Compose composable.

- `alfred create composable`
- fill out prompts
- resulting composable created:

```kotlin
package tmp.com.example.foobar

import androidx.compose.foundation.*;
import androidx.compose.foundation.layout.*;
import androidx.compose.runtime.Composable;
import androidx.compose.runtime.collectAsState;
import androidx.compose.runtime.getValue;
import tmp.UiState;
import tmp.ViewStateViewModel;

@Composable
fun Foobar(
    viewModel: ViewStateViewModel<*>,
    onViewCreated: (
        vm: ViewStateViewModel<*>
    ) {
    val viewState by viewModel.viewState.collectAsState()

    DisposableEffect(Unit) {
        onViewCreated(viewModel)
        onDispose {
            viewModel.cleanup()
        }
    }

    BaseContainer {
        when (state = viewState.state) {
            is UiState.Loading -> Loading()
            is UiState.Error -> Error()
            is UiState.Success -> Content()
        }
    }
}

@Composable
private fun BaseContainer(content: @Composable () -> Unit) {
    Box {
        content()
    }
}

@Composable
private fun Loading() {
    TODO("not implemented")
}

@Composable
private fun Error() {
    TODO("not implemented")
}

@Composable
private fun Content() {
    TODO("not implemented")
}
```