const TAB_NAMES = ["a", "b", "c", "d"];

let current_tab;
function open_tab(tab_id) {
	for(let i in TAB_NAMES) {
		document.getElementById("sel_" + TAB_NAMES[i]).style.opacity = "0%";
		document.getElementById("tab_" + TAB_NAMES[i]).style.display = "none";
	}

	document.getElementById("sel_" + TAB_NAMES[tab_id]).style.opacity = "100%";
	document.getElementById("tab_" + TAB_NAMES[tab_id]).style.display = "inline";
	current_tab = tab_id;
}

function format_text(input_text) {
	let output = document.createElement("span");
	let target = output;
	let target_text = "";
	let inside_ruby = false;
	let current_marker = 0;
	for (let i in input_text.content) {
		if (current_marker < input_text.markers.length && input_text.markers[current_marker].offset <= i) {
			if (input_text.markers[current_marker].op == "ruby-base" && !inside_ruby) {
				target.appendChild(document.createTextNode(target_text));

				target_text = "";
				target = document.createElement("span");
				target.classList = "ruby";
				inside_ruby = true;
			} else if (input_text.markers[current_marker].op == "ruby-end" && inside_ruby) {
				target.appendChild(document.createTextNode(target_text));

				output.appendChild(target);
				target_text = "";
				target = output;
				inside_ruby = false;
			} else if (input_text.markers[current_marker].op == "linebreak") {
				target.appendChild(document.createTextNode(target_text));
				target.appendChild(document.createElement("br"));
				target_text = "";
			}
			current_marker++;
		}
		target_text += input_text.content[i];
	}
	if (target_text != "") {
		target.appendChild(document.createTextNode(target_text));
	}
	if (inside_ruby) {
		output.appendChild(target);
	}
	return output;
}

function add_tweep(tweep) {
	let tweep_div = document.createElement("div");
	tweep_div.classList = "tweep";

	let author_div = document.createElement("div");
	author_div.classList = "author";

	let author_img = document.createElement("img");
	author_img.src = "img/pfp" + tweep.pfp_id.toString().padStart(2, "0") + ".png";

	let author_name_div = document.createElement("div");
	author_name_div.appendChild(format_text(tweep.author_username));
	author_name_div.appendChild(document.createElement("br"));
	author_name_div.appendChild(format_text(tweep.author_realname));

	author_div.appendChild(author_img);
	author_div.appendChild(author_name_div);

	let text_div = document.createElement("div");
	text_div.appendChild(format_text(tweep.content));

	let details_div = document.createElement("div");
	details_div.classList = "details";
	let details_span = document.createElement("span");
	details_span.appendChild(document.createTextNode("Today"));
	details_div.appendChild(details_span);

	tweep_div.appendChild(author_div);
	tweep_div.appendChild(text_div);
	tweep_div.appendChild(details_div);

	document.getElementById("tab_" + TAB_NAMES[tweep.tab]).prepend(tweep_div);
}

function clear_tweeps() {
	for(let i in TAB_NAMES) {
		document.getElementById("tab_" + TAB_NAMES[i]).innerHTML = "";
	}
}

let touch_start = 0;
document.addEventListener("touchstart", function(e) {
	touch_start = e.changedTouches[0].screenX;
});
document.addEventListener("touchend", function(e) {
	let dx = e.changedTouches[0].screenX - touch_start;
	if (Math.abs(dx) > 100) {
		if (dx > 0 && current_tab > 0) { open_tab(current_tab - 1); }
		if (dx < 0 && current_tab < 3) { open_tab(current_tab + 1); }
	}
});

document.addEventListener('DOMContentLoaded', function() {
	open_tab(0);
	let websocket = new WebSocket("ws://" + location.host + "/websocket");
	websocket.onmessage = function(e) {
		let message = JSON.parse(e.data);
		if (message.type == "clear") {
			clear_tweeps();
		} else if (message.type == "tweep") {
			add_tweep(message.tweep);
		} else {
			alert("Unknown message : " + e.data);
		}
	};
	// TODO : support idle
	// TODO : replies
	websocket.onclose = function(e) {
		alert("WebSocket closed : " + e.code + " " + e.reason);
	};
	websocket.onerror = function(e) {
		alert("WebSocket error !");
	};
});
