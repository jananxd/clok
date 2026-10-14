# The purpose of this file is to have something we can use as note taking inside the editor and to know what specific things we need to do in a task. Make the checklist smaller task as possible so that we don't get bored because it's too hard.

## August 22, 2026
- [x] Create a box for the initial view, it should have the instructions on how to quit the program and how to create a new task.
- [x] Establish the [TEA](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) architecture, make it as minimal as possible.
- [x] Create a local list for the tasks.
- [x] Create a view to create task, it should only ask for name and description (optional) then when submitted we want to render it to the list and go back to the initial view.

## August 30, 2026
- [x] Setup a localdb like SQLite so we can save the tasks and time entry records in there.
- [x] we wanna save the tasks to the local DB
- [ ] Do the time entry functionality with stopwatch. Just a simple one wherein when they enter specific task, they can start+stop the timer. When the timer is active, we should have the stopwatch running on the task list view.

## Sept 12, 2026
- [x] Implement the start timer
- [x] Make sure on the initial run of the app, we will get the tasks active timers.

# Oct 6, 2026
- [ ] add a key binding to initiate the login flow.
- [ ] create a way to produce an error, maybe introduce a new string in the model and a Popup component to show the error, allow a keybindings to also close the the popup, it will be nice if this Popup is a global component so we can reuse it on every error that we have and they close it using `ESC`.
- [ ] Implement auth

1. so what we wanna do is to do is call the init endpoint, the init endpoint will generate a device code and return a URL
2. we should figure out how to open the OS browser and open the URL
3. do a poll to the /auth/verify endpoint, maybe have a 15mins timeout, once we receive a 200 status code in that endpoint, then we can assume that it's verified and it will return a user object.
