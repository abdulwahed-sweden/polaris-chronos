(function () {
  'use strict';

  // ── DOM refs ─────────────────────────────────────────────────
  var cityInput = document.getElementById('city-input');
  var dateInput = document.getElementById('date-input');
  var goBtn = document.getElementById('go-btn');
  var emptyState = document.getElementById('empty-state');
  var errorState = document.getElementById('error-state');
  var errorMsg = document.getElementById('error-msg');
  var loadingState = document.getElementById('loading-state');
  var results = document.getElementById('results');
  var autocompleteList = document.getElementById('autocomplete-list');

  var stratBtns = document.querySelectorAll('.strat-btn');
  var currentStrategy = 'projected45';
  var cities = [];
  var selectedIndex = -1;
  var lastData = null;
  var countdownInterval = null;

  // Set default date
  dateInput.value = new Date().toISOString().split('T')[0];

  // Load cities
  fetch('/api/cities')
    .then(function (r) { return r.json(); })
    .then(function (data) { cities = data; })
    .catch(function () {});

  // ── Strategy toggle ─────────────────────────────────────────
  stratBtns.forEach(function (btn) {
    btn.addEventListener('click', function () {
      stratBtns.forEach(function (b) { b.classList.remove('active'); });
      btn.classList.add('active');
      currentStrategy = btn.dataset.value;
      if (lastData) fetchTimes();
    });
  });

  // ── Autocomplete ───────────────────────────────────────────
  cityInput.addEventListener('input', function () {
    var val = this.value.trim().toLowerCase();
    selectedIndex = -1;
    if (val.length < 1) { closeAC(); return; }

    var matches = cities.filter(function (c) {
      return c.name.toLowerCase().indexOf(val) !== -1;
    }).slice(0, 8);

    if (matches.length === 0) { closeAC(); return; }

    autocompleteList.innerHTML = '';
    matches.forEach(function (city) {
      var item = document.createElement('div');
      item.className = 'autocomplete-item';
      item.innerHTML = '<span>' + capitalize(city.name) + '</span>' +
        '<span class="country-code">' + city.country + '</span>';
      item.addEventListener('mousedown', function (e) {
        e.preventDefault();
        cityInput.value = capitalize(city.name);
        closeAC();
        fetchTimes();
      });
      autocompleteList.appendChild(item);
    });
    autocompleteList.classList.add('active');
  });

  cityInput.addEventListener('keydown', function (e) {
    var items = autocompleteList.querySelectorAll('.autocomplete-item');
    if (!items.length && e.key !== 'Enter') return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      highlightItem(items);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      highlightItem(items);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedIndex >= 0 && items.length) {
        items[selectedIndex].dispatchEvent(new Event('mousedown'));
      } else {
        fetchTimes();
      }
    }
  });

  cityInput.addEventListener('blur', function () {
    setTimeout(closeAC, 150);
  });

  function closeAC() {
    autocompleteList.classList.remove('active');
    autocompleteList.innerHTML = '';
    selectedIndex = -1;
  }

  function highlightItem(items) {
    items.forEach(function (el) { el.classList.remove('selected'); });
    if (selectedIndex >= 0 && selectedIndex < items.length) {
      items[selectedIndex].classList.add('selected');
    }
  }

  // ── Coordinates copy ────────────────────────────────────────
  document.getElementById('loc-coords').addEventListener('click', function () {
    var text = this.textContent;
    if (!text) return;
    var el = this;
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text).then(function () {
        el.classList.add('copied');
        var original = el.textContent;
        el.textContent = 'Copied!';
        setTimeout(function () {
          el.textContent = original;
          el.classList.remove('copied');
        }, 1200);
      });
    }
  });

  // ── Fetch ──────────────────────────────────────────────────
  goBtn.addEventListener('click', fetchTimes);

  function fetchTimes() {
    var city = cityInput.value.trim();
    if (!city) { showError('Please enter a city name.'); return; }

    showLoading();

    // Step 1: Resolve city
    var resolveParams = new URLSearchParams();
    resolveParams.set('query', city);

    fetch('/api/resolve?' + resolveParams.toString())
      .then(function (r) {
        if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'City not found'); });
        return r.json();
      })
      .then(function (loc) {
        // Step 2: Fetch times using resolved coordinates
        var timesParams = new URLSearchParams();
        timesParams.set('lat', loc.lat);
        timesParams.set('lon', loc.lon);
        timesParams.set('tz', loc.tz);
        if (dateInput.value) timesParams.set('date', dateInput.value);
        timesParams.set('strategy', currentStrategy);

        return Promise.all([
          Promise.resolve(loc),
          fetch('/api/times?' + timesParams.toString()).then(function (r) {
            if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'Computation failed'); });
            return r.json();
          })
        ]);
      })
      .then(function (pair) {
        renderResults(pair[0], pair[1]);
      })
      .catch(function (err) {
        showError(err.message);
      });
  }

  // ── State management ───────────────────────────────────────

  function showLoading() {
    emptyState.style.display = 'none';
    errorState.style.display = 'none';
    results.style.display = 'none';
    loadingState.style.display = 'block';
  }

  function showError(msg) {
    emptyState.style.display = 'none';
    loadingState.style.display = 'none';
    results.style.display = 'none';
    errorMsg.textContent = msg;
    errorState.style.display = 'block';
  }

  function showResults() {
    emptyState.style.display = 'none';
    errorState.style.display = 'none';
    loadingState.style.display = 'none';
    results.style.display = 'block';
  }

  // ── Render ─────────────────────────────────────────────────

  function renderResults(loc, data) {
    lastData = data;
    showResults();

    // Location banner
    var name = capitalize(loc.name);
    if (loc.country_code) name += ', ' + loc.country_code;
    document.getElementById('loc-name').textContent = name;
    document.getElementById('loc-tz').textContent = loc.tz;
    document.getElementById('loc-date').textContent = data.date;
    document.getElementById('loc-coords').textContent =
      data.location.latitude.toFixed(2) + ', ' + data.location.longitude.toFixed(2);

    var stateEl = document.getElementById('day-state');
    stateEl.textContent = data.state;
    stateEl.className = 'day-state ' + data.state.toLowerCase().replace(/\s+/g, '');

    // Strategy note
    var stratNote = document.getElementById('strategy-note');
    var stratText = document.getElementById('strategy-note-text');
    var stratIcon = stratNote.querySelector('.strategy-note-icon');
    var allStandard = checkAllStandard(data);

    if (allStandard) {
      stratNote.style.display = 'flex';
      stratNote.className = 'strategy-note all-standard';
      stratIcon.textContent = '\u2714';
      stratText.textContent = 'All times are astronomically computed (real sun positions).';
    } else {
      stratNote.style.display = 'flex';
      stratNote.className = 'strategy-note has-projected';
      stratIcon.textContent = '\u26A0';
      stratText.textContent = 'Some times are projected or virtual due to extreme latitude conditions.';
    }

    // Prayer table
    var tbody = document.getElementById('prayer-tbody');
    tbody.innerHTML = '';
    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    var methodIcons = {
      'Standard': '\u{1F7E2}',
      'Projected': '\u{1F7E0}',
      'Virtual': '\u{1F7E3}',
      'None': '\u2B1C'
    };

    var currentPrayer = detectCurrentPrayer(data);

    prayers.forEach(function (name) {
      var ev = data.events[name];
      var tr = document.createElement('tr');

      // Maghrib special highlight
      if (name === 'maghrib') {
        tr.classList.add('maghrib-row');
      }

      if (currentPrayer && currentPrayer.current === name) {
        tr.classList.add('current-prayer');
      }

      // Name
      var tdName = document.createElement('td');
      tdName.textContent = capitalize(name);
      tr.appendChild(tdName);

      // Time
      var tdTime = document.createElement('td');
      tdTime.className = 'time-cell';
      if (ev.time) {
        tdTime.textContent = formatTime(ev.time) + (ev.next_day ? ' +1' : '');
      } else {
        tdTime.textContent = '---';
        tdTime.classList.add('no-time');
      }
      tr.appendChild(tdTime);

      // Method
      var tdMethod = document.createElement('td');
      var methodClass = ev.method.toLowerCase();
      var icon = methodIcons[ev.method] || '';
      tdMethod.innerHTML = '<span class="method-badge ' + methodClass + '">' +
        '<span class="method-icon">' + icon + '</span>' +
        ev.method + '</span>';
      tr.appendChild(tdMethod);

      // Confidence
      var tdConf = document.createElement('td');
      var confPct = Math.round(ev.confidence * 100);
      var confClass = ev.confidence >= 0.9 ? 'high' : ev.confidence >= 0.6 ? 'medium' : 'low';
      tdConf.innerHTML = '<div class="confidence-cell">' +
        '<div class="confidence-bar"><div class="confidence-fill ' + confClass +
        '" style="width:' + confPct + '%"></div></div>' +
        '<span class="confidence-value">' + ev.confidence.toFixed(1) + '</span></div>';
      tr.appendChild(tdConf);

      tbody.appendChild(tr);
    });

    // Solar info
    document.getElementById('solar-max').textContent = data.solar.max_altitude.toFixed(2) + '\u00B0';
    document.getElementById('solar-min').textContent = data.solar.min_altitude.toFixed(2) + '\u00B0';
    document.getElementById('solar-peak').textContent = data.solar.peak_utc;
    document.getElementById('solar-nadir').textContent = data.solar.nadir_utc;

    // Meta
    document.getElementById('meta-strategy').textContent = data.gap_strategy;
    document.getElementById('meta-source').textContent = data.location.source;
    document.getElementById('meta-confidence').textContent = data.location.resolved_confidence.toFixed(2);

    // Now bar + countdown
    updateNowBar(data, currentPrayer);
  }

  // ── Strategy check ──────────────────────────────────────────

  function checkAllStandard(data) {
    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    for (var i = 0; i < prayers.length; i++) {
      if (data.events[prayers[i]].method !== 'Standard') return false;
    }
    return true;
  }

  // ── Current prayer detection ───────────────────────────────

  function detectCurrentPrayer(data) {
    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    var now = new Date();
    var todayStr = now.toISOString().split('T')[0];

    // Only show current prayer if we're viewing today
    if (data.date !== todayStr) return null;

    var nowMinutes = now.getHours() * 60 + now.getMinutes();
    var times = [];

    prayers.forEach(function (name) {
      var ev = data.events[name];
      if (ev.time && !ev.next_day) {
        var parts = ev.time.split(':');
        var mins = parseInt(parts[0]) * 60 + parseInt(parts[1]);
        times.push({ name: name, minutes: mins });
      }
    });

    if (times.length === 0) return null;

    times.sort(function (a, b) { return a.minutes - b.minutes; });

    var current = null;
    var next = null;

    for (var i = times.length - 1; i >= 0; i--) {
      if (nowMinutes >= times[i].minutes) {
        current = times[i].name;
        next = i + 1 < times.length ? times[i + 1] : null;
        break;
      }
    }

    if (!current && times.length > 0) {
      // Before first prayer
      current = null;
      next = times[0];
    }

    return {
      current: current,
      next: next ? next.name : null,
      nextMinutes: next ? next.minutes : null
    };
  }

  function updateNowBar(data, currentPrayer) {
    var nowBar = document.getElementById('now-bar');
    if (countdownInterval) {
      clearInterval(countdownInterval);
      countdownInterval = null;
    }

    if (!currentPrayer || !currentPrayer.next) {
      nowBar.style.display = 'none';
      return;
    }

    nowBar.style.display = 'flex';
    document.getElementById('now-prayer').textContent =
      currentPrayer.current ? capitalize(currentPrayer.current) : 'Before ' + capitalize(currentPrayer.next);
    document.getElementById('now-next').textContent =
      capitalize(currentPrayer.next) + ' next';

    function updateCountdown() {
      var now = new Date();
      var nowMins = now.getHours() * 60 + now.getMinutes();
      var diff = currentPrayer.nextMinutes - nowMins;
      var countdownEl = document.getElementById('now-countdown');

      if (diff <= 0) {
        countdownEl.textContent = 'Now';
        countdownEl.classList.remove('urgent');
        if (countdownInterval) clearInterval(countdownInterval);
        // Refresh after a minute
        setTimeout(function () { fetchTimes(); }, 60000);
        return;
      }

      // Urgent: less than 60 minutes
      if (diff <= 60) {
        countdownEl.classList.add('urgent');
      } else {
        countdownEl.classList.remove('urgent');
      }

      var h = Math.floor(diff / 60);
      var m = diff % 60;
      countdownEl.textContent = (h > 0 ? h + 'h ' : '') + m + 'm';
    }

    updateCountdown();
    countdownInterval = setInterval(updateCountdown, 30000);
  }

  // ── Helpers ────────────────────────────────────────────────

  function capitalize(s) {
    if (!s) return '';
    return s.replace(/\b\w/g, function (c) { return c.toUpperCase(); });
  }

  function formatTime(t) {
    return t.substring(0, 5);
  }
})();
