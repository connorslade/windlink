import Toybox.Application;
import Toybox.Lang;
import Toybox.WatchUi;

class App extends Application.AppBase {
    var boat = new Boat();

    function initialize() {
        AppBase.initialize();
    }

    function onStart(state as Dictionary?) as Void {
        self.boat.initialize();
    }

    function onStop(state as Dictionary?) as Void {}

    function getInitialView() as [Views] or [Views, InputDelegates] {
        return [new ConnectingView()];
    }
}

function getApp() as App {
    return Application.getApp() as App;
}
