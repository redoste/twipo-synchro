const TAB_NAMES = ["a", "b", "c", "d"];

let current_tab;
function open_tab(tab_id, update_scroll) {
	for(let i in TAB_NAMES) {
		document.getElementById("sel_" + TAB_NAMES[i]).style.opacity = "0%";
		document.getElementById("tab_" + TAB_NAMES[i]).style.display = "none";
	}

	current_tab = tab_id;
	document.getElementById("sel_" + TAB_NAMES[tab_id]).style.opacity = "100%";

	let target = document.getElementById("tab_" + TAB_NAMES[tab_id]);
	target.style.display = "inline";
	if (update_scroll) {
		on_tab_scroll({target: target});
	}
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

const DAY_TEXTS = ["Yesterday", "Today"];

function add_tweep(tweep) {
	let tab = document.getElementById("tab_" + TAB_NAMES[tweep.tab]);
	// This is an ugly workaround since `scrollTop` and `offsetHeight` will not work with `display: none`
	// The elements are not visible, of course they have a height of 0px and can't be scrolled
	let old_display = tab.style.display;
	tab.style.display = "inline";
	let old_scroll = tab.scrollTop;

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
	let posted_today = tweep.post_date == window.game_date;
	details_span.innerText = DAY_TEXTS[Number(posted_today)];
	details_span.classList = "details_date"
	details_span.dataset.date = tweep.post_date;
	details_div.appendChild(details_span);

	let replies_div = null;
	if (tweep.replies.length > 0) {
		let replies_div_id = "replies_" + tweep.id.toString().padStart(8, "0");
		let reply_button_div_id = "reply_button_" + tweep.id.toString().padStart(8, "0");

		let reply_button_div = document.createElement("div");
		reply_button_div.classList = "details_button";
		reply_button_div.id = reply_button_div_id;
		reply_button_div.dataset.replies_div_id = replies_div_id;
		reply_button_div.onclick = function() {
			let replies_div = document.getElementById(this.dataset.replies_div_id);
			let hidden = replies_div.style.display == "none";
			replies_div.style.display = hidden ? "block" : "none";
		};

		let reply_button_img = document.createElement("img");
		reply_button_img.src = "img/reply.png";
		reply_button_div.appendChild(reply_button_img);

		details_div.appendChild(reply_button_div);

		replies_div = document.createElement("div");
		replies_div.className = "replies";
		replies_div.style.display = "none";
		replies_div.id = replies_div_id;
		for (let index in tweep.replies) {
			let reply_div = document.createElement("div");
			reply_div.classList = "reply";
			reply_div.appendChild(format_text(tweep.replies[index]));

			let reply_details_div = document.createElement("div");
			reply_details_div.classList = "details";
			reply_details_div.appendChild(document.createElement("span"));

			let send_button_div = document.createElement("div");
			send_button_div.classList = "details_button";
			send_button_div.dataset.replies_div_id = replies_div_id;
			send_button_div.dataset.tweep_id = tweep.id;
			send_button_div.dataset.reply_id = index;
			send_button_div.onclick = function() {
				if (send_reply(Number(this.dataset.tweep_id), Number(this.dataset.reply_id))) {
					document.getElementById(this.dataset.replies_div_id).style.display = "none";
				}
			};

			let send_button_img = document.createElement("img");
			send_button_img.src = "img/send.png";
			send_button_div.appendChild(send_button_img);

			reply_details_div.appendChild(send_button_div);

			reply_div.appendChild(reply_details_div);
			replies_div.appendChild(reply_div);
		}

		if (!tweep.reply_possible) {
			reply_button_div.style.display = "none";
		}
	}

	tweep_div.appendChild(author_div);
	tweep_div.appendChild(text_div);
	if (replies_div != null) {
		tweep_div.appendChild(replies_div);
	}
	tweep_div.appendChild(details_div);

	let new_tweeps_notification_div = tab.getElementsByClassName("new_tweeps_notification")[0];
	tab.insertBefore(tweep_div, new_tweeps_notification_div.nextSibling);

	tab.scrollTop = old_scroll + tweep_div.offsetHeight;
	if (tab.scrollTop > 0) {
		tab.getElementsByClassName("new_tweeps_notification_span")[0].style.opacity = "100%";
	}

	tab.style.display = old_display;
}

function send_reply(tweep_id, reply_id) {
	if (window.websocket.readyState != window.WebSocket.OPEN) {
		return false;
	}

	window.websocket.send(JSON.stringify({
		type: "reply",
		tweep_id: tweep_id,
		reply_id: reply_id,
	}));
	return true;
}

function on_tab_scroll(e) {
	// Sometimes the browser will call this after we add a Tweep and the tab is still hidden
	// This will break everything
	if (e.target.style.display != "inline") {
		return;
	}

	if (e.target.scrollTop == 0) {
		e.target.getElementsByClassName("new_tweeps_notification_span")[0].style.opacity = "0";
	}
}

function clear_tweeps() {
	for(let i in TAB_NAMES) {
		let tab = document.getElementById("tab_" + TAB_NAMES[i]);
		tab.innerHTML = '<div class="new_tweeps_notification"><span class="new_tweeps_notification_span">↑ See new Tweeps</span></div>';
		tab.scrollTop = 0;
		if(tab.onscroll === null) { tab.onscroll = on_tab_scroll; }
	}
}

function set_reply_possible(tweep_id, possible) {
	let reply_button_div_id = "reply_button_" + tweep_id.toString().padStart(8, "0");
	let replies_div_id = "replies_" + tweep_id.toString().padStart(8, "0");

	let reply_button_div = document.getElementById(reply_button_div_id);
	let replies_div = document.getElementById(replies_div_id);

	if (replies_div != null && !possible) { replies_div.style.display = "none"; }
	if (reply_button_div != null) { reply_button_div.style.display = possible ? "block" : "none"; }
}

function update_date() {
	let spans = document.getElementsByClassName("details_date");
	for (let i = 0; i < spans.length; i++) {
		let today = Number(spans[i].dataset.date) == window.game_date;
		spans[i].innerText = DAY_TEXTS[Number(today)];
	}
}

function connect_websocket() {
	clear_tweeps();
	console.log("(re)connecting to websocket");
	let websocket = new WebSocket("ws://" + location.host + "/websocket");
	window.websocket = websocket;
	window.websocketfailed = false;
	websocket.onmessage = function(e) {
		let message = JSON.parse(e.data);
		if (message.type == "clear") {
			clear_tweeps();
		} else if (message.type == "tweep") {
			add_tweep(message.tweep);
		} else if (message.type == "set_reply_possible") {
			set_reply_possible(message.tweep_id, message.possible);
		} else if (message.type == "date") {
			window.game_date = message.date;
			update_date();
		} else {
			alert("Unknown message : " + e.data);
		}
	};
	websocket.onclose = function(e) {
		alert("WebSocket closed : " + e.code + " " + e.reason + "\n" + "You can refresh the page to reconnect.");
		window.websocketfailed = true;
	};
}

let touch_start = 0;
document.addEventListener("touchstart", function(e) {
	touch_start = e.changedTouches[0].screenX;
});
document.addEventListener("touchend", function(e) {
	let dx = e.changedTouches[0].screenX - touch_start;
	if (Math.abs(dx) > 100) {
		if (dx > 0 && current_tab > 0) { open_tab(current_tab - 1, true); }
		if (dx < 0 && current_tab < 3) { open_tab(current_tab + 1, true); }
	}
});

document.addEventListener('DOMContentLoaded', function() {
	open_tab(0, false);
	connect_websocket();

	setInterval(function() {
		if (window.websocket.readyState != window.WebSocket.OPEN && !window.websocketfailed) {
			// We make sure the old websocket does not interfere with the new one
			window.websocket.onmessage = null;
			window.websocket.onclose = null;
			window.websocket.close();
			window.websocket = null;

			connect_websocket();
		}
	}, 500);
});
