(function () {
  'use strict';

  // ── Ramadan 1447 AH ─────────────────────────────────────
  var RAMADAN_START = new Date(2026, 1, 17); // Feb 17, 2026
  var RAMADAN_DAYS = 30;
  var RAMADAN_YEAR = '1447 AH';
  var DEFAULT_CITY = 'Tromsø';

  // ── DOM refs ────────────────────────────────────────────
  var cityInput = document.getElementById('city-input');
  var goBtn = document.getElementById('go-btn');
  var errorState = document.getElementById('error-state');
  var errorMsg = document.getElementById('error-msg');
  var loadingState = document.getElementById('loading-state');
  var calendarView = document.getElementById('calendar-view');
  var dayView = document.getElementById('day-view');
  var autocompleteList = document.getElementById('autocomplete-list');

  // ── State ───────────────────────────────────────────────
  var cities = [];
  var selectedIndex = -1;
  var currentLoc = null;
  var countdownInterval = null;

  // ── Routing ─────────────────────────────────────────────
  var route = window.location.pathname === '/day' ? 'day' : 'calendar';

  // ── Init ────────────────────────────────────────────────
  fetch('/api/cities', { cache: 'no-store' })
    .then(function (r) { return r.json(); })
    .then(function (data) { cities = data; })
    .catch(function () {});

  if (route === 'calendar') {
    initCalendar();
  } else {
    initDayView();
  }

  // ═══════════════════════════════════════════════════════
  //  CALENDAR VIEW
  // ═══════════════════════════════════════════════════════

  function initCalendar() {
    dayView.style.display = 'none';

    var params = new URLSearchParams(window.location.search);
    var city = params.get('city') || DEFAULT_CITY;
    cityInput.value = city;

    goBtn.addEventListener('click', function () {
      var c = cityInput.value.trim();
      if (c) loadCalendarForCity(c);
    });

    setupAutocomplete(function (name) {
      cityInput.value = name;
      loadCalendarForCity(name);
    });

    loadCalendarForCity(city);
  }

  function loadCalendarForCity(city) {
    showLoading();
    hideDisambiguation();

    fetch('/api/resolve?query=' + encodeURIComponent(city), { cache: 'no-store' })
      .then(function (r) {
        if (r.status === 300) {
          return r.json().then(function (j) {
            showDisambiguation(j, function (loc) {
              currentLoc = loc;
              fetchRamadanMonth(loc);
            });
            return null;
          });
        }
        if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'City not found'); });
        return r.json();
      })
      .then(function (loc) {
        if (!loc) return;
        currentLoc = loc;
        fetchRamadanMonth(loc);
      })
      .catch(function (err) {
        showError(err.message);
      });
  }

  function fetchRamadanMonth(loc) {
    // Ramadan 1447 spans Feb + Mar 2026
    var p1 = fetch('/api/month?lat=' + loc.lat + '&lon=' + loc.lon +
      '&tz=' + encodeURIComponent(loc.tz) + '&year=2026&month=2', { cache: 'no-store' })
      .then(function (r) { return r.json(); });

    var p2 = fetch('/api/month?lat=' + loc.lat + '&lon=' + loc.lon +
      '&tz=' + encodeURIComponent(loc.tz) + '&year=2026&month=3', { cache: 'no-store' })
      .then(function (r) { return r.json(); });

    Promise.all([p1, p2])
      .then(function (results) {
        var allDays = {};
        results[0].forEach(function (d) { allDays[d.date] = d; });
        results[1].forEach(function (d) { allDays[d.date] = d; });

        var ramadanData = [];
        for (var i = 0; i < RAMADAN_DAYS; i++) {
          var date = new Date(RAMADAN_START);
          date.setDate(date.getDate() + i);
          var dateStr = isoDate(date);
          if (allDays[dateStr]) {
            ramadanData.push({ date: dateStr, ramadanDay: i + 1, data: allDays[dateStr] });
          }
        }

        renderCalendar(loc, ramadanData);
      })
      .catch(function (err) {
        showError('Failed to load calendar: ' + err.message);
      });
  }

  function renderCalendar(loc, days) {
    hideLoading();
    calendarView.style.display = 'block';

    // Location banner
    var displayName = capitalize(loc.name);
    if (loc.country) displayName += ', ' + loc.country;
    else if (loc.country_code) displayName += ', ' + loc.country_code;

    document.getElementById('cal-loc-name').textContent = displayName;
    document.getElementById('cal-loc-tz').textContent = loc.tz_label || loc.tz;
    document.getElementById('cal-loc-coords').textContent =
      loc.formatted_coords || formatCoords(loc.lat, loc.lon);

    // Ramadan header
    document.getElementById('ramadan-title').textContent = 'Ramadan ' + RAMADAN_YEAR;
    var endDate = new Date(RAMADAN_START);
    endDate.setDate(endDate.getDate() + RAMADAN_DAYS - 1);
    document.getElementById('ramadan-dates').textContent =
      displayDate(RAMADAN_START) + ' \u2014 ' + displayDate(endDate);

    // Calendar grid
    var grid = document.getElementById('calendar-grid');
    grid.innerHTML = '';
    var todayStr = isoDate(new Date());

    days.forEach(function (day) {
      var card = document.createElement('div');
      card.className = 'day-card';
      if (day.date === todayStr) card.classList.add('today');

      var fajrEv = day.data.events.fajr;
      var maghribEv = day.data.events.maghrib;
      var fajrTime = fajrEv.time ? fmtTime(fajrEv.time) : '---';
      var iftarTime = maghribEv.time ? fmtTime(maghribEv.time) : '---';
      var fajrMethod = (fajrEv.method || 'Standard').toLowerCase();
      var iftarMethod = (maghribEv.method || 'Standard').toLowerCase();

      var dateObj = new Date(day.date + 'T12:00:00');
      var dayName = dateObj.toLocaleDateString('en', { weekday: 'short' });
      var monthDay = dateObj.toLocaleDateString('en', { month: 'short', day: 'numeric' });

      card.innerHTML =
        '<div class="card-header">' +
          '<span class="card-ramadan-day">Ramadan ' + day.ramadanDay + '</span>' +
          '<span class="card-date">' + dayName + ', ' + monthDay + '</span>' +
        '</div>' +
        '<div class="card-times">' +
          '<div class="card-time-row">' +
            '<span class="card-label">Fajr</span>' +
            '<span class="card-time">' + fajrTime + '</span>' +
            '<span class="method-dot ' + fajrMethod + '"></span>' +
          '</div>' +
          '<div class="card-time-row iftar-row">' +
            '<span class="card-label">Iftar</span>' +
            '<span class="card-time iftar-time">' + iftarTime + '</span>' +
            '<span class="method-dot ' + iftarMethod + '"></span>' +
          '</div>' +
        '</div>';

      card.addEventListener('click', (function (d) {
        return function () {
          window.location.href = '/day?date=' + d.date +
            '&lat=' + loc.lat + '&lon=' + loc.lon +
            '&tz=' + encodeURIComponent(loc.tz) +
            '&name=' + encodeURIComponent(loc.name) +
            (loc.country ? '&country=' + encodeURIComponent(loc.country) : '') +
            (loc.country_code ? '&cc=' + encodeURIComponent(loc.country_code) : '');
        };
      })(day));

      grid.appendChild(card);
    });

    // Now intelligence
    setupNowBar(days);
  }

  // ── Now bar ─────────────────────────────────────────────

  function setupNowBar(days) {
    var nowBar = document.getElementById('now-bar');
    if (countdownInterval) {
      clearInterval(countdownInterval);
      countdownInterval = null;
    }

    var todayStr = isoDate(new Date());
    var todayData = null;
    for (var i = 0; i < days.length; i++) {
      if (days[i].date === todayStr) {
        todayData = days[i].data;
        break;
      }
    }

    if (!todayData) {
      nowBar.style.display = 'none';
      return;
    }

    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    var times = [];
    prayers.forEach(function (name) {
      var ev = todayData.events[name];
      if (ev && ev.time && !ev.next_day) {
        var parts = ev.time.split(':');
        var totalSecs = parseInt(parts[0]) * 3600 + parseInt(parts[1]) * 60 +
          (parts.length > 2 ? parseInt(parts[2]) : 0);
        times.push({ name: name, seconds: totalSecs });
      }
    });

    if (times.length === 0) {
      nowBar.style.display = 'none';
      return;
    }

    times.sort(function (a, b) { return a.seconds - b.seconds; });

    function update() {
      var now = new Date();
      var nowSecs = now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();

      var current = null;
      var next = null;

      for (var i = times.length - 1; i >= 0; i--) {
        if (nowSecs >= times[i].seconds) {
          current = times[i];
          next = i + 1 < times.length ? times[i + 1] : null;
          break;
        }
      }

      if (!current && times.length > 0) {
        next = times[0];
      }

      if (!next) {
        nowBar.style.display = 'none';
        return;
      }

      nowBar.style.display = 'flex';
      document.getElementById('now-prayer').textContent =
        current ? capitalize(current.name) : 'Before ' + capitalize(next.name);
      document.getElementById('now-next').textContent = capitalize(next.name);

      var diff = next.seconds - nowSecs;
      if (diff < 0) diff = 0;
      document.getElementById('now-countdown').textContent = fmtCountdown(diff);
    }

    update();
    countdownInterval = setInterval(update, 1000);
  }

  // ═══════════════════════════════════════════════════════
  //  DAY DETAIL VIEW
  // ═══════════════════════════════════════════════════════

  function initDayView() {
    calendarView.style.display = 'none';

    var params = new URLSearchParams(window.location.search);
    var date = params.get('date');
    var lat = parseFloat(params.get('lat'));
    var lon = parseFloat(params.get('lon'));
    var tz = params.get('tz');
    var name = params.get('name') || '';
    var country = params.get('country') || '';
    var cc = params.get('cc') || '';

    if (!date || isNaN(lat) || isNaN(lon) || !tz) {
      showError('Missing parameters. Use the calendar to select a day.');
      return;
    }

    // Back link
    document.getElementById('back-link').href = '/?city=' + encodeURIComponent(name);

    // Location banner
    var displayName = capitalize(name);
    if (country) displayName += ', ' + country;
    else if (cc) displayName += ', ' + cc;

    document.getElementById('day-loc-name').textContent = displayName;
    document.getElementById('day-loc-tz').textContent = tz + ' (Local Time)';
    document.getElementById('day-loc-coords').textContent = formatCoords(lat, lon);

    // Day header
    var dateObj = new Date(date + 'T12:00:00');
    var ramadanDay = Math.round((dateObj - RAMADAN_START) / 86400000) + 1;
    var dayLabel = (ramadanDay >= 1 && ramadanDay <= 30) ?
      'Ramadan ' + ramadanDay : dateObj.toLocaleDateString('en', { month: 'long', day: 'numeric', year: 'numeric' });

    document.getElementById('day-title').textContent = dayLabel + ' \u2014 ' + displayName;
    document.getElementById('day-subtitle').textContent =
      dateObj.toLocaleDateString('en', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });

    // City input
    cityInput.value = capitalize(name);

    // Search → go back to calendar
    goBtn.addEventListener('click', function () {
      var c = cityInput.value.trim();
      if (c) window.location.href = '/?city=' + encodeURIComponent(c);
    });

    setupAutocomplete(function (cityName) {
      window.location.href = '/?city=' + encodeURIComponent(cityName);
    });

    // Strategy toggle
    var currentStrategy = 'projected45';
    var stratBtns = document.querySelectorAll('.strat-btn');
    stratBtns.forEach(function (btn) {
      btn.addEventListener('click', function () {
        stratBtns.forEach(function (b) { b.classList.remove('active'); });
        btn.classList.add('active');
        currentStrategy = btn.dataset.value;
        fetchDayData(lat, lon, tz, date, currentStrategy);
      });
    });

    fetchDayData(lat, lon, tz, date, currentStrategy);
  }

  function fetchDayData(lat, lon, tz, date, strategy) {
    showLoading();
    dayView.style.display = 'none';

    var url = '/api/times?lat=' + lat + '&lon=' + lon +
      '&tz=' + encodeURIComponent(tz) + '&date=' + date + '&strategy=' + strategy;

    fetch(url, { cache: 'no-store' })
      .then(function (r) {
        if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'Failed'); });
        return r.json();
      })
      .then(function (data) {
        renderDayDetail(data);
      })
      .catch(function (err) {
        showError(err.message);
      });
  }

  function renderDayDetail(data) {
    hideLoading();
    dayView.style.display = 'block';

    // Strategy note
    var stratNote = document.getElementById('day-strategy-note');
    var stratText = document.getElementById('day-strategy-text');
    var stratIcon = stratNote.querySelector('.strategy-note-icon');
    var allStandard = true;
    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];

    prayers.forEach(function (name) {
      if (data.events[name].method !== 'Standard') allStandard = false;
    });

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
    var methodIcons = {
      'Standard': '\u{1F7E2}',
      'Projected': '\u{1F7E0}',
      'Virtual': '\u{1F7E3}',
      'None': '\u2B1C'
    };

    var todayStr = isoDate(new Date());
    var isToday = data.date === todayStr;
    var currentPrayer = isToday ? detectCurrentPrayer(data) : null;

    prayers.forEach(function (name) {
      var ev = data.events[name];
      var tr = document.createElement('tr');

      if (name === 'maghrib') tr.classList.add('maghrib-row');
      if (currentPrayer && currentPrayer.current === name) tr.classList.add('current-prayer');

      var tdName = document.createElement('td');
      tdName.textContent = capitalize(name);
      tr.appendChild(tdName);

      var tdTime = document.createElement('td');
      tdTime.className = 'time-cell';
      if (ev.time) {
        tdTime.textContent = fmtTime(ev.time) + (ev.next_day ? ' +1' : '');
      } else {
        tdTime.textContent = '---';
        tdTime.classList.add('no-time');
      }
      tr.appendChild(tdTime);

      var tdMethod = document.createElement('td');
      var mClass = ev.method.toLowerCase();
      var icon = methodIcons[ev.method] || '';
      tdMethod.innerHTML = '<span class="method-badge ' + mClass + '">' +
        '<span class="method-icon">' + icon + '</span>' + ev.method + '</span>';
      tr.appendChild(tdMethod);

      var tdConf = document.createElement('td');
      var pct = Math.round(ev.confidence * 100);
      var cClass = ev.confidence >= 0.9 ? 'high' : ev.confidence >= 0.6 ? 'medium' : 'low';
      tdConf.innerHTML = '<div class="confidence-cell">' +
        '<div class="confidence-bar"><div class="confidence-fill ' + cClass +
        '" style="width:' + pct + '%"></div></div>' +
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
  }

  function detectCurrentPrayer(data) {
    var prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    var now = new Date();
    var nowMins = now.getHours() * 60 + now.getMinutes();
    var times = [];

    prayers.forEach(function (name) {
      var ev = data.events[name];
      if (ev.time && !ev.next_day) {
        var parts = ev.time.split(':');
        times.push({ name: name, minutes: parseInt(parts[0]) * 60 + parseInt(parts[1]) });
      }
    });

    if (times.length === 0) return null;
    times.sort(function (a, b) { return a.minutes - b.minutes; });

    var current = null;
    var next = null;

    for (var i = times.length - 1; i >= 0; i--) {
      if (nowMins >= times[i].minutes) {
        current = times[i].name;
        next = i + 1 < times.length ? times[i + 1] : null;
        break;
      }
    }

    if (!current && times.length > 0) {
      next = times[0];
    }

    return { current: current, next: next ? next.name : null };
  }

  // ═══════════════════════════════════════════════════════
  //  SHARED: AUTOCOMPLETE
  // ═══════════════════════════════════════════════════════

  function setupAutocomplete(onSelect) {
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
          onSelect(capitalize(city.name));
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
          var city = cityInput.value.trim();
          if (city) {
            if (route === 'calendar') {
              loadCalendarForCity(city);
            } else {
              window.location.href = '/?city=' + encodeURIComponent(city);
            }
          }
        }
      }
    });

    cityInput.addEventListener('blur', function () {
      setTimeout(closeAC, 150);
    });
  }

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

  // ═══════════════════════════════════════════════════════
  //  SHARED: DISAMBIGUATION
  // ═══════════════════════════════════════════════════════

  function showDisambiguation(data, onSelect) {
    hideLoading();
    var container = document.getElementById('disambiguation');
    if (!container) {
      container = document.createElement('div');
      container.id = 'disambiguation';
      container.className = 'disambiguation';
      loadingState.parentNode.insertBefore(container, loadingState.nextSibling);
    }

    var html = '<p>Multiple locations found for "<strong>' + escapeHtml(data.query) + '</strong>":</p>';
    data.options.forEach(function (opt) {
      html += '<button class="disambig-btn" data-cc="' + escapeHtml(opt.country_code) + '" ' +
        'data-lat="' + opt.lat + '" data-lon="' + opt.lon + '" data-tz="' + escapeHtml(opt.tz) + '" ' +
        'data-name="' + escapeHtml(opt.name) + '">' +
        '<span class="disambig-name">' + escapeHtml(opt.name) + '</span>' +
        '<span class="disambig-detail">' + escapeHtml(opt.country) + ' \u00B7 ' + escapeHtml(opt.tz) + '</span>' +
        '</button>';
    });

    container.innerHTML = html;
    container.style.display = 'block';

    container.querySelectorAll('.disambig-btn').forEach(function (btn) {
      btn.addEventListener('click', function () {
        hideDisambiguation();
        var loc = {
          name: btn.dataset.name,
          lat: parseFloat(btn.dataset.lat),
          lon: parseFloat(btn.dataset.lon),
          tz: btn.dataset.tz,
          country_code: btn.dataset.cc,
          country: btn.querySelector('.disambig-detail').textContent.split(' \u00B7 ')[0],
          tz_label: btn.dataset.tz + ' (Local Time)',
          formatted_coords: formatCoords(parseFloat(btn.dataset.lat), parseFloat(btn.dataset.lon)),
          source: 'Nominatim',
          confidence: 0.9,
        };
        onSelect(loc);
      });
    });
  }

  function hideDisambiguation() {
    var el = document.getElementById('disambiguation');
    if (el) el.style.display = 'none';
  }

  // ═══════════════════════════════════════════════════════
  //  SHARED: STATE MANAGEMENT
  // ═══════════════════════════════════════════════════════

  function showLoading() {
    errorState.style.display = 'none';
    calendarView.style.display = 'none';
    dayView.style.display = 'none';
    hideDisambiguation();
    loadingState.style.display = 'block';
  }

  function hideLoading() {
    loadingState.style.display = 'none';
  }

  function showError(msg) {
    loadingState.style.display = 'none';
    calendarView.style.display = 'none';
    dayView.style.display = 'none';
    hideDisambiguation();
    errorMsg.textContent = msg;
    errorState.style.display = 'block';
  }

  // ═══════════════════════════════════════════════════════
  //  SHARED: HELPERS
  // ═══════════════════════════════════════════════════════

  function capitalize(s) {
    if (!s) return '';
    return s.replace(/\b\w/g, function (c) { return c.toUpperCase(); });
  }

  function fmtTime(t) {
    return t.substring(0, 5);
  }

  function pad2(n) {
    return n < 10 ? '0' + n : '' + n;
  }

  function isoDate(d) {
    return d.getFullYear() + '-' + pad2(d.getMonth() + 1) + '-' + pad2(d.getDate());
  }

  function displayDate(d) {
    return d.toLocaleDateString('en', { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function formatCoords(lat, lon) {
    var ns = lat >= 0 ? 'N' : 'S';
    var ew = lon >= 0 ? 'E' : 'W';
    return Math.abs(lat).toFixed(2) + '\u00B0' + ns + ', ' + Math.abs(lon).toFixed(2) + '\u00B0' + ew;
  }

  function fmtCountdown(secs) {
    var h = Math.floor(secs / 3600);
    var m = Math.floor((secs % 3600) / 60);
    var s = secs % 60;
    return pad2(h) + ':' + pad2(m) + ':' + pad2(s);
  }

  function escapeHtml(s) {
    if (!s) return '';
    var div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
  }

  // ── Coords copy (event delegation) ─────────────────────
  document.addEventListener('click', function (e) {
    if (e.target.classList.contains('loc-coords')) {
      var text = e.target.textContent;
      if (!text || !navigator.clipboard) return;
      var el = e.target;
      navigator.clipboard.writeText(text).then(function () {
        el.classList.add('copied');
        var orig = el.textContent;
        el.textContent = 'Copied!';
        setTimeout(function () {
          el.textContent = orig;
          el.classList.remove('copied');
        }, 1200);
      });
    }
  });

})();
