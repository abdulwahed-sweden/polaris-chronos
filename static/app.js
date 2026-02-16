(function () {
  'use strict';

  // ═══════════════════════════════════════════════════════════
  //  1. CONFIG & CONSTANTS
  // ═══════════════════════════════════════════════════════════

  var RAMADAN_START = new Date(2026, 1, 17); // Feb 17, 2026
  var RAMADAN_DAYS = 30;
  var RAMADAN_YEAR = '1447 AH';
  var DEFAULT_CITY = 'Troms\u00f8';

  var PRAYER_NAMES = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];

  var METHOD_ICONS = {
    'Standard': '\u{1F7E2}',
    'Projected': '\u{1F7E0}',
    'Virtual': '\u{1F7E3}',
    'None': '\u2B1C'
  };

  var METHOD_COLORS = {
    'Standard': '#16A34A',
    'Projected': '#D97706',
    'Virtual': '#7C3AED',
    'None': '#9CA3AF'
  };

  // ═══════════════════════════════════════════════════════════
  //  2. STATE
  // ═══════════════════════════════════════════════════════════

  var state = {
    cities: [],
    currentLoc: null,
    currentRoute: 'calendar',
    calendarData: null,
    dayData: null,
    selectedIndex: -1,
    countdownInterval: null
  };

  // ═══════════════════════════════════════════════════════════
  //  3. DOM REFS
  // ═══════════════════════════════════════════════════════════

  var $ = function (id) { return document.getElementById(id); };
  var cityInput = $('city-input');
  var dateInput = $('date-input');
  var goBtn = $('go-btn');
  var errorState = $('error-state');
  var errorMsg = $('error-msg');
  var loadingState = $('loading-state');
  var calendarView = $('calendar-view');
  var dayView = $('day-view');
  var docsView = $('docs-view');
  var autocompleteList = $('autocomplete-list');
  var nowDashboard = $('now-dashboard');
  var searchSection = $('search-section');

  // ═══════════════════════════════════════════════════════════
  //  4. ROUTER
  // ═══════════════════════════════════════════════════════════

  function getRoute() {
    var path = window.location.pathname;
    if (path === '/docs') return 'docs';
    if (path === '/day') return 'day';
    return 'calendar';
  }

  function setActiveNav(route) {
    document.querySelectorAll('.polaris-nav .nav-link').forEach(function (link) {
      link.classList.remove('active');
      if (link.dataset.route === route) {
        link.classList.add('active');
      }
    });
  }

  function initRouter() {
    // SPA-style navigation
    document.querySelectorAll('.polaris-nav .nav-link').forEach(function (link) {
      link.addEventListener('click', function (e) {
        e.preventDefault();
        var href = this.getAttribute('href');
        var route = this.dataset.route;
        if (route === state.currentRoute && route !== 'day') return;
        window.history.pushState({}, '', href);
        navigateToRoute(route);
      });
    });

    window.addEventListener('popstate', function () {
      navigateToRoute(getRoute());
    });
  }

  function navigateToRoute(route) {
    state.currentRoute = route;
    setActiveNav(route);
    hideAll();

    if (route === 'calendar') {
      searchSection.style.display = '';
      initCalendar();
    } else if (route === 'day') {
      searchSection.style.display = '';
      initDayView();
    } else if (route === 'docs') {
      searchSection.style.display = 'none';
      nowDashboard.style.display = 'none';
      renderDocs();
    }
  }

  function hideAll() {
    errorState.style.display = 'none';
    loadingState.style.display = 'none';
    calendarView.style.display = 'none';
    dayView.style.display = 'none';
    docsView.style.display = 'none';
    hideDisambiguation();
  }

  // ═══════════════════════════════════════════════════════════
  //  5. API LAYER
  // ═══════════════════════════════════════════════════════════

  var api = {
    resolve: function (query) {
      return fetch('/api/resolve?query=' + encodeURIComponent(query), { cache: 'no-store' })
        .then(handleResponse);
    },
    times: function (params) {
      var q = 'lat=' + params.lat + '&lon=' + params.lon +
        '&tz=' + encodeURIComponent(params.tz) + '&date=' + params.date +
        '&strategy=' + (params.strategy || 'projected45');
      return fetch('/api/times?' + q, { cache: 'no-store' })
        .then(function (r) {
          if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'Failed'); });
          return r.json();
        });
    },
    month: function (params) {
      var q = 'lat=' + params.lat + '&lon=' + params.lon +
        '&tz=' + encodeURIComponent(params.tz) +
        '&year=' + params.year + '&month=' + params.month;
      return fetch('/api/month?' + q, { cache: 'no-store' })
        .then(function (r) {
          if (!r.ok) throw new Error('Failed to fetch month data');
          return r.json();
        });
    },
    cities: function () {
      return fetch('/api/cities', { cache: 'no-store' })
        .then(function (r) { return r.json(); });
    }
  };

  function handleResponse(r) {
    if (r.status === 300) {
      return r.json().then(function (j) {
        var err = new Error('DISAMBIGUATE');
        err.data = j;
        throw err;
      });
    }
    if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'City not found'); });
    return r.json();
  }

  // ═══════════════════════════════════════════════════════════
  //  6. UTILITIES
  // ═══════════════════════════════════════════════════════════

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

  function displayName(loc) {
    var name = capitalize(loc.name);
    if (loc.country) name += ', ' + loc.country;
    else if (loc.country_code) name += ', ' + loc.country_code;
    return name;
  }

  // ═══════════════════════════════════════════════════════════
  //  7. AUTOCOMPLETE
  // ═══════════════════════════════════════════════════════════

  function setupAutocomplete(onSelect) {
    cityInput.addEventListener('input', function () {
      var val = this.value.trim().toLowerCase();
      state.selectedIndex = -1;
      if (val.length < 1) { closeAC(); return; }

      var matches = state.cities.filter(function (c) {
        return c.name.toLowerCase().indexOf(val) !== -1;
      }).slice(0, 8);

      if (matches.length === 0) { closeAC(); return; }

      autocompleteList.innerHTML = '';
      matches.forEach(function (city) {
        var item = document.createElement('div');
        item.className = 'autocomplete-item';
        item.innerHTML = '<span>' + escapeHtml(capitalize(city.name)) + '</span>' +
          '<span class="country-badge">' + escapeHtml(city.country) + '</span>';
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
        state.selectedIndex = Math.min(state.selectedIndex + 1, items.length - 1);
        highlightItem(items);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        state.selectedIndex = Math.max(state.selectedIndex - 1, 0);
        highlightItem(items);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (state.selectedIndex >= 0 && items.length) {
          items[state.selectedIndex].dispatchEvent(new Event('mousedown'));
        } else {
          var city = cityInput.value.trim();
          if (city) {
            if (state.currentRoute === 'calendar') {
              loadCalendarForCity(city);
            } else {
              window.history.pushState({}, '', '/?city=' + encodeURIComponent(city));
              navigateToRoute('calendar');
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
    state.selectedIndex = -1;
  }

  function highlightItem(items) {
    items.forEach(function (el) { el.classList.remove('selected'); });
    if (state.selectedIndex >= 0 && state.selectedIndex < items.length) {
      items[state.selectedIndex].classList.add('selected');
    }
  }

  // ═══════════════════════════════════════════════════════════
  //  8. NOW DASHBOARD
  // ═══════════════════════════════════════════════════════════

  function updateNowDashboard(days) {
    if (state.countdownInterval) {
      clearInterval(state.countdownInterval);
      state.countdownInterval = null;
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
      nowDashboard.style.display = 'none';
      return;
    }

    var times = [];
    PRAYER_NAMES.forEach(function (name) {
      var ev = todayData.events[name];
      if (ev && ev.time && !ev.next_day) {
        var parts = ev.time.split(':');
        var totalSecs = parseInt(parts[0]) * 3600 + parseInt(parts[1]) * 60 +
          (parts.length > 2 ? parseInt(parts[2]) : 0);
        times.push({ name: name, seconds: totalSecs });
      }
    });

    if (times.length === 0) {
      nowDashboard.style.display = 'none';
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
        nowDashboard.style.display = 'none';
        return;
      }

      nowDashboard.style.display = 'block';
      $('now-prayer-name').textContent =
        current ? capitalize(current.name) : 'Before ' + capitalize(next.name);
      $('now-next-name').textContent = capitalize(next.name);

      var diff = next.seconds - nowSecs;
      if (diff < 0) diff = 0;
      $('now-countdown').textContent = fmtCountdown(diff);
    }

    update();
    state.countdownInterval = setInterval(update, 1000);
  }

  // ═══════════════════════════════════════════════════════════
  //  9. CALENDAR VIEW
  // ═══════════════════════════════════════════════════════════

  function initCalendar() {
    dayView.style.display = 'none';
    docsView.style.display = 'none';

    var params = new URLSearchParams(window.location.search);
    var city = params.get('city') || DEFAULT_CITY;
    cityInput.value = city;

    goBtn.onclick = function () {
      if (dateInput.value && state.currentLoc) {
        navigateToDay(dateInput.value, state.currentLoc);
        return;
      }
      var c = cityInput.value.trim();
      if (c) loadCalendarForCity(c);
    };

    dateInput.onchange = function () {
      if (dateInput.value && state.currentLoc) {
        navigateToDay(dateInput.value, state.currentLoc);
      }
    };

    setupAutocomplete(function (name) {
      cityInput.value = name;
      loadCalendarForCity(name);
    });

    loadCalendarForCity(city);
  }

  function loadCalendarForCity(city) {
    showLoading();
    hideDisambiguation();

    api.resolve(city)
      .then(function (loc) {
        fetchRamadanMonth(loc);
      })
      .catch(function (err) {
        if (err.message === 'DISAMBIGUATE') {
          showDisambiguation(err.data, function (loc) {
            fetchRamadanMonth(loc);
          });
        } else {
          showError(err.message);
        }
      });
  }

  function fetchRamadanMonth(loc) {
    var p1 = api.month({ lat: loc.lat, lon: loc.lon, tz: loc.tz, year: 2026, month: 2 });
    var p2 = api.month({ lat: loc.lat, lon: loc.lon, tz: loc.tz, year: 2026, month: 3 });

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

        state.calendarData = ramadanData;
        renderCalendar(loc, ramadanData);
      })
      .catch(function (err) {
        showError('Failed to load calendar: ' + err.message);
      });
  }

  function navigateToDay(date, loc) {
    var href = '/day?date=' + date +
      '&lat=' + loc.lat + '&lon=' + loc.lon +
      '&tz=' + encodeURIComponent(loc.tz) +
      '&name=' + encodeURIComponent(loc.name) +
      (loc.country ? '&country=' + encodeURIComponent(loc.country) : '') +
      (loc.country_code ? '&cc=' + encodeURIComponent(loc.country_code) : '');
    window.history.pushState({}, '', href);
    navigateToRoute('day');
  }

  function renderCalendar(loc, days) {
    hideLoading();
    calendarView.style.display = 'block';
    state.currentLoc = loc;

    // Location banner
    $('cal-loc-name').textContent = displayName(loc);
    $('cal-loc-tz').textContent = loc.tz_label || loc.tz;
    $('cal-loc-coords').textContent = loc.formatted_coords || formatCoords(loc.lat, loc.lon);

    // Ramadan header
    $('ramadan-title').textContent = 'Ramadan ' + RAMADAN_YEAR;
    var endDate = new Date(RAMADAN_START);
    endDate.setDate(endDate.getDate() + RAMADAN_DAYS - 1);
    $('ramadan-dates').textContent = displayDate(RAMADAN_START) + ' \u2014 ' + displayDate(endDate);

    // Calendar table
    var tbody = $('calendar-tbody');
    tbody.innerHTML = '';
    var todayStr = isoDate(new Date());

    days.forEach(function (day) {
      var tr = document.createElement('tr');
      if (day.date === todayStr) tr.className = 'today-row';

      var dateObj = new Date(day.date + 'T12:00:00');
      var dayName = dateObj.toLocaleDateString('en', { weekday: 'short' });
      var monthDay = dateObj.toLocaleDateString('en', { month: 'short', day: 'numeric' });

      // Day number
      var tdDay = document.createElement('td');
      tdDay.className = 'cal-day-num';
      tdDay.textContent = day.ramadanDay;
      tr.appendChild(tdDay);

      // Date
      var tdDate = document.createElement('td');
      tdDate.className = 'cal-date';
      tdDate.textContent = dayName + ', ' + monthDay;
      tr.appendChild(tdDate);

      // Prayer times with method badges
      PRAYER_NAMES.forEach(function (key) {
        var ev = day.data.events[key];
        var td = document.createElement('td');
        td.className = 'time-cell';
        if (key === 'maghrib') td.classList.add('iftar-cell');

        if (ev && ev.time) {
          var timeSpan = document.createElement('span');
          timeSpan.textContent = fmtTime(ev.time) + ' ';
          td.appendChild(timeSpan);

          var method = (ev.method || 'Standard').toLowerCase();
          if (method !== 'standard') {
            var badge = document.createElement('span');
            var badgeClass = method === 'virtual' ? 'virtual-h' : method;
            badge.className = 'method-badge ' + badgeClass;
            badge.style.fontSize = '0.65rem';
            badge.style.padding = '0.05rem 0.35rem';
            badge.textContent = method.charAt(0).toUpperCase();
            td.appendChild(badge);
          }
        } else {
          td.textContent = '---';
          td.classList.add('no-time');
        }

        tr.appendChild(td);
      });

      tr.addEventListener('click', (function (d) {
        return function () { navigateToDay(d.date, loc); };
      })(day));

      tbody.appendChild(tr);
    });

    // Now dashboard
    updateNowDashboard(days);
  }

  // ═══════════════════════════════════════════════════════════
  //  10. DAY VIEW
  // ═══════════════════════════════════════════════════════════

  function initDayView() {
    calendarView.style.display = 'none';
    docsView.style.display = 'none';

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

    // Location banner
    var locDisplayName = capitalize(name);
    if (country) locDisplayName += ', ' + country;
    else if (cc) locDisplayName += ', ' + cc;

    $('day-loc-name').textContent = locDisplayName;
    $('day-loc-tz').textContent = tz + ' (Local Time)';
    $('day-loc-coords').textContent = formatCoords(lat, lon);

    // Day header
    var dateObj = new Date(date + 'T12:00:00');
    var ramadanDay = Math.round((dateObj - RAMADAN_START) / 86400000) + 1;
    var dayLabel = (ramadanDay >= 1 && ramadanDay <= 30) ?
      'Ramadan ' + ramadanDay : dateObj.toLocaleDateString('en', { month: 'long', day: 'numeric', year: 'numeric' });

    $('day-title').textContent = dayLabel + ' \u2014 ' + locDisplayName;
    $('day-subtitle').textContent =
      dateObj.toLocaleDateString('en', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });

    // City input
    cityInput.value = capitalize(name);

    // Search -> go back to calendar
    goBtn.onclick = function () {
      var c = cityInput.value.trim();
      if (c) {
        window.history.pushState({}, '', '/?city=' + encodeURIComponent(c));
        navigateToRoute('calendar');
      }
    };

    setupAutocomplete(function (cityName) {
      window.history.pushState({}, '', '/?city=' + encodeURIComponent(cityName));
      navigateToRoute('calendar');
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

    api.times({ lat: lat, lon: lon, tz: tz, date: date, strategy: strategy })
      .then(function (data) {
        state.dayData = data;
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
    var stratNote = $('day-strategy-note');
    var stratText = $('day-strategy-text');
    var stratIcon = stratNote.querySelector('.strategy-note-icon');
    var allStandard = true;

    PRAYER_NAMES.forEach(function (name) {
      if (data.events[name].method !== 'Standard') allStandard = false;
    });

    if (allStandard) {
      stratNote.style.display = 'flex';
      stratNote.className = 'strategy-note all-standard mb-3';
      stratIcon.textContent = '\u2714';
      stratText.textContent = 'All times are astronomically computed (real sun positions).';
    } else {
      stratNote.style.display = 'flex';
      stratNote.className = 'strategy-note has-projected mb-3';
      stratIcon.textContent = '\u26A0';
      stratText.textContent = 'Some times are projected or virtual due to extreme latitude conditions.';
    }

    // Prayer table
    var tbody = $('prayer-tbody');
    tbody.innerHTML = '';

    var todayStr = isoDate(new Date());
    var isToday = data.date === todayStr;
    var currentPrayer = isToday ? detectCurrentPrayer(data) : null;

    PRAYER_NAMES.forEach(function (name) {
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
      var badgeClass = mClass === 'virtual' ? 'virtual-h' : mClass;
      var icon = METHOD_ICONS[ev.method] || '';
      tdMethod.innerHTML = '<span class="method-badge ' + badgeClass + '">' +
        '<span class="method-icon">' + icon + '</span>' + escapeHtml(ev.method) + '</span>';
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
    $('solar-max').textContent = data.solar.max_altitude.toFixed(2) + '\u00B0';
    $('solar-min').textContent = data.solar.min_altitude.toFixed(2) + '\u00B0';
    $('solar-peak').textContent = data.solar.peak_utc;
    $('solar-nadir').textContent = data.solar.nadir_utc;

    // Meta
    $('meta-strategy').textContent = data.gap_strategy;
    $('meta-source').textContent = data.location.source;
    $('meta-confidence').textContent = data.location.resolved_confidence.toFixed(2);

    // Horizon Dial
    renderHorizonDial(data);
  }

  function detectCurrentPrayer(data) {
    var now = new Date();
    var nowMins = now.getHours() * 60 + now.getMinutes();
    var times = [];

    PRAYER_NAMES.forEach(function (name) {
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

  // ═══════════════════════════════════════════════════════════
  //  11. HORIZON DIAL SVG
  // ═══════════════════════════════════════════════════════════

  function renderHorizonDial(data) {
    var container = $('horizon-dial-container');
    var dial = $('horizon-dial');
    dial.innerHTML = '';

    // Check if we have enough data
    var prayerTimes = [];
    PRAYER_NAMES.forEach(function (name) {
      var ev = data.events[name];
      if (ev && ev.time && !ev.next_day) {
        var parts = ev.time.split(':');
        var hours = parseInt(parts[0]) + parseInt(parts[1]) / 60;
        prayerTimes.push({ name: name, hours: hours, method: ev.method });
      }
    });

    if (prayerTimes.length < 2) {
      // Polar night / not enough data
      container.style.display = 'block';
      dial.innerHTML = '<div style="text-align:center;padding:2rem;color:#9CA3AF;">' +
        'Insufficient solar data for horizon visualization (polar conditions).</div>';
      return;
    }

    container.style.display = 'block';

    var W = 600, H = 320;
    var CX = 300, CY = 260;
    var R = 220;

    var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
    svg.setAttribute('preserveAspectRatio', 'xMidYMid meet');

    // Background gradient
    var defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    var grad = document.createElementNS('http://www.w3.org/2000/svg', 'linearGradient');
    grad.setAttribute('id', 'skyGrad');
    grad.setAttribute('x1', '0'); grad.setAttribute('y1', '0');
    grad.setAttribute('x2', '0'); grad.setAttribute('y2', '1');
    var stop1 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
    stop1.setAttribute('offset', '0%');
    stop1.setAttribute('style', 'stop-color:rgba(22,163,74,0.04)');
    var stop2 = document.createElementNS('http://www.w3.org/2000/svg', 'stop');
    stop2.setAttribute('offset', '100%');
    stop2.setAttribute('style', 'stop-color:rgba(248,250,251,0)');
    grad.appendChild(stop1);
    grad.appendChild(stop2);
    defs.appendChild(grad);
    svg.appendChild(defs);

    // Sky area
    var skyPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    skyPath.setAttribute('d', 'M ' + (CX - R) + ' ' + CY + ' A ' + R + ' ' + R + ' 0 0 1 ' + (CX + R) + ' ' + CY + ' Z');
    skyPath.setAttribute('fill', 'url(#skyGrad)');
    svg.appendChild(skyPath);

    // Horizon line
    var horizon = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    horizon.setAttribute('x1', CX - R - 20);
    horizon.setAttribute('y1', CY);
    horizon.setAttribute('x2', CX + R + 20);
    horizon.setAttribute('y2', CY);
    horizon.setAttribute('class', 'dial-horizon');
    svg.appendChild(horizon);

    // Horizon labels
    var eastLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    eastLabel.setAttribute('x', CX - R - 15);
    eastLabel.setAttribute('y', CY + 18);
    eastLabel.setAttribute('text-anchor', 'middle');
    eastLabel.setAttribute('class', 'dial-note');
    eastLabel.textContent = 'E';
    svg.appendChild(eastLabel);

    var westLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    westLabel.setAttribute('x', CX + R + 15);
    westLabel.setAttribute('y', CY + 18);
    westLabel.setAttribute('text-anchor', 'middle');
    westLabel.setAttribute('class', 'dial-note');
    westLabel.textContent = 'W';
    svg.appendChild(westLabel);

    // Arc guide lines
    [0.25, 0.5, 0.75].forEach(function (frac) {
      var r = R * frac;
      var arc = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      arc.setAttribute('d', 'M ' + (CX - r) + ' ' + CY + ' A ' + r + ' ' + r + ' 0 0 1 ' + (CX + r) + ' ' + CY);
      arc.setAttribute('class', 'dial-arc');
      arc.setAttribute('stroke-dasharray', '4 6');
      svg.appendChild(arc);
    });

    // Compute angle range: map fajr-to-isha across 180 degrees (left to right)
    prayerTimes.sort(function (a, b) { return a.hours - b.hours; });
    var firstHour = prayerTimes[0].hours;
    var lastHour = prayerTimes[prayerTimes.length - 1].hours;
    var range = lastHour - firstHour;
    if (range <= 0) range = 1;

    // Sun path arc
    var pathPoints = [];
    for (var t = 0; t <= 1; t += 0.02) {
      var angle = Math.PI * (1 - t); // left (pi) to right (0)
      var alt = Math.sin(Math.PI * t); // parabolic altitude
      var pr = R * (0.15 + 0.85 * alt);
      var px = CX + pr * Math.cos(angle);
      var py = CY - pr * Math.sin(angle);
      pathPoints.push((t === 0 ? 'M' : 'L') + ' ' + px.toFixed(1) + ' ' + py.toFixed(1));
    }
    var sunPath = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    sunPath.setAttribute('d', pathPoints.join(' '));
    sunPath.setAttribute('class', 'dial-sun-path');
    svg.appendChild(sunPath);

    // Prayer markers
    prayerTimes.forEach(function (pt) {
      var frac = (pt.hours - firstHour) / range;
      var angle = Math.PI * (1 - frac);
      var alt = Math.sin(Math.PI * frac);
      var mr = R * (0.15 + 0.85 * alt);
      var mx = CX + mr * Math.cos(angle);
      var my = CY - mr * Math.sin(angle);

      var g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
      g.setAttribute('class', 'dial-marker');

      var circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      circle.setAttribute('cx', mx);
      circle.setAttribute('cy', my);
      circle.setAttribute('r', 6);
      circle.setAttribute('fill', METHOD_COLORS[pt.method] || '#64748b');
      circle.setAttribute('stroke', '#fff');
      circle.setAttribute('stroke-width', 1);
      g.appendChild(circle);

      var label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      label.setAttribute('x', mx);
      label.setAttribute('y', my - 14);
      label.setAttribute('text-anchor', 'middle');
      label.setAttribute('class', 'dial-marker-label');
      label.textContent = capitalize(pt.name);
      g.appendChild(label);

      // Time label below
      var timeLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      timeLabel.setAttribute('x', mx);
      timeLabel.setAttribute('y', my + 20);
      timeLabel.setAttribute('text-anchor', 'middle');
      timeLabel.setAttribute('class', 'dial-note');
      var hh = Math.floor(pt.hours);
      var mm = Math.round((pt.hours - hh) * 60);
      timeLabel.textContent = pad2(hh) + ':' + pad2(mm);
      g.appendChild(timeLabel);

      svg.appendChild(g);
    });

    // Current sun position (if today)
    var todayStr = isoDate(new Date());
    if (data.date === todayStr) {
      var now = new Date();
      var nowHours = now.getHours() + now.getMinutes() / 60;
      if (nowHours >= firstHour && nowHours <= lastHour) {
        var frac = (nowHours - firstHour) / range;
        var angle = Math.PI * (1 - frac);
        var alt = Math.sin(Math.PI * frac);
        var sr = R * (0.15 + 0.85 * alt);
        var sx = CX + sr * Math.cos(angle);
        var sy = CY - sr * Math.sin(angle);

        var sunCircle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        sunCircle.setAttribute('cx', sx);
        sunCircle.setAttribute('cy', sy);
        sunCircle.setAttribute('r', 8);
        sunCircle.setAttribute('class', 'dial-sun dial-sun-pulse');
        svg.appendChild(sunCircle);
      }
    }

    dial.appendChild(svg);
  }

  // ═══════════════════════════════════════════════════════════
  //  12. DOCS VIEW
  // ═══════════════════════════════════════════════════════════

  function renderDocs() {
    docsView.style.display = 'block';

    var endpoints = [
      {
        path: '/api/resolve',
        desc: 'Resolve a city name to coordinates, timezone, and metadata. Returns a single match or HTTP 300 with multiple candidates for disambiguation.',
        params: [
          { name: 'query', type: 'string', required: true, desc: 'City name to search (e.g. "Stockholm", "Medina")' },
          { name: 'country', type: 'string', required: false, desc: 'ISO 3166-1 alpha-2 country hint (e.g. "SA", "US")' }
        ],
        curl: "curl 'http://localhost:3000/api/resolve?query=stockholm'",
        js: "const res = await fetch('/api/resolve?query=stockholm');\nconst loc = await res.json();\nconsole.log(loc.name, loc.lat, loc.lon);"
      },
      {
        path: '/api/times',
        desc: 'Compute prayer times for a specific location and date. Supports both city-based and coordinate-based queries.',
        params: [
          { name: 'city', type: 'string', required: false, desc: 'City name (alternative to lat/lon)' },
          { name: 'lat', type: 'number', required: false, desc: 'Latitude (-90 to 90)' },
          { name: 'lon', type: 'number', required: false, desc: 'Longitude (-180 to 180)' },
          { name: 'tz', type: 'string', required: false, desc: 'IANA timezone (e.g. "Europe/Stockholm")' },
          { name: 'date', type: 'string', required: false, desc: 'Date in YYYY-MM-DD format (defaults to today)' },
          { name: 'strategy', type: 'string', required: false, desc: '"projected45" (default) or "strict"' },
          { name: 'country', type: 'string', required: false, desc: 'Country hint for city disambiguation' }
        ],
        curl: "curl 'http://localhost:3000/api/times?city=stockholm&date=2026-03-01'",
        js: "const res = await fetch('/api/times?city=stockholm&date=2026-03-01');\nconst data = await res.json();\nconsole.log(data.events.fajr.time);"
      },
      {
        path: '/api/month',
        desc: 'Compute prayer times for every day in a given month. Useful for generating Imsakia calendars.',
        params: [
          { name: 'city', type: 'string', required: false, desc: 'City name (alternative to lat/lon)' },
          { name: 'lat', type: 'number', required: false, desc: 'Latitude (-90 to 90)' },
          { name: 'lon', type: 'number', required: false, desc: 'Longitude (-180 to 180)' },
          { name: 'tz', type: 'string', required: false, desc: 'IANA timezone' },
          { name: 'year', type: 'number', required: false, desc: 'Year (defaults to current)' },
          { name: 'month', type: 'number', required: false, desc: 'Month 1-12 (defaults to current)' },
          { name: 'strategy', type: 'string', required: false, desc: '"projected45" (default) or "strict"' },
          { name: 'country', type: 'string', required: false, desc: 'Country hint' }
        ],
        curl: "curl 'http://localhost:3000/api/month?city=stockholm&year=2026&month=2'",
        js: "const res = await fetch('/api/month?city=stockholm&year=2026&month=2');\nconst days = await res.json();\ndays.forEach(d => console.log(d.date, d.events.fajr.time));"
      },
      {
        path: '/api/cities',
        desc: 'List all built-in cities with their country codes. Useful for autocomplete or dropdown implementations.',
        params: [],
        curl: "curl 'http://localhost:3000/api/cities'",
        js: "const res = await fetch('/api/cities');\nconst cities = await res.json();\nconsole.log(cities.length, 'cities available');"
      }
    ];

    var html = '<div class="docs-header">' +
      '<h1>API Documentation</h1>' +
      '<p>Polaris Chronos provides a RESTful API for computing prayer times at any location on Earth, including polar regions with adaptive compensation.</p>' +
      '</div>';

    endpoints.forEach(function (ep) {
      html += '<div class="polaris-card docs-endpoint mb-4">' +
        '<div class="endpoint-badge get">GET ' + escapeHtml(ep.path) + '</div>' +
        '<p class="endpoint-description">' + escapeHtml(ep.desc) + '</p>';

      if (ep.params.length > 0) {
        html += '<table class="docs-param-table"><thead><tr>' +
          '<th>Parameter</th><th>Type</th><th>Required</th><th>Description</th>' +
          '</tr></thead><tbody>';
        ep.params.forEach(function (p) {
          html += '<tr>' +
            '<td><code>' + escapeHtml(p.name) + '</code></td>' +
            '<td>' + escapeHtml(p.type) + '</td>' +
            '<td>' + (p.required ? 'Yes' : 'No') + '</td>' +
            '<td>' + escapeHtml(p.desc) + '</td>' +
            '</tr>';
        });
        html += '</tbody></table>';
      }

      html += '<div class="docs-code-label">curl</div>' +
        '<div class="docs-code-block">' + escapeHtml(ep.curl) + '</div>' +
        '<div class="docs-code-label">JavaScript</div>' +
        '<div class="docs-code-block">' + escapeHtml(ep.js) + '</div>' +
        '</div>';
    });

    // Concepts section
    html += '<div class="docs-concepts">' +
      '<h3>Concepts</h3>' +
      '<div class="polaris-card mb-3">' +
      '<p class="mb-3"><span class="concept-badge">DayState</span> Describes the solar condition for a given day at the location.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>Normal</strong> \u2014 Standard sunrise/sunset cycle. All times are astronomically computed.</li>' +
      '<li><strong>WhiteNight</strong> \u2014 Sun dips below horizon but not far enough for twilight-based events.</li>' +
      '<li><strong>PolarDay</strong> \u2014 Sun never sets (midnight sun). Sunset/twilight events require compensation.</li>' +
      '<li><strong>PolarNight</strong> \u2014 Sun never rises. Sunrise/daytime events require compensation.</li>' +
      '</ul>' +
      '<p class="mb-3"><span class="concept-badge">EventMethod</span> How each prayer time was determined.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>Standard</strong> \u2014 Real astronomical calculation from sun position.</li>' +
      '<li><strong>Projected</strong> \u2014 Interpolated from the nearest latitude (45\u00B0) where the event occurs naturally.</li>' +
      '<li><strong>Virtual</strong> \u2014 Computed using virtual horizon techniques for extreme conditions.</li>' +
      '<li><strong>None</strong> \u2014 Event cannot be computed (e.g. no sunset in polar day with strict strategy).</li>' +
      '</ul>' +
      '<p class="mb-3"><span class="concept-badge">GapStrategy</span> How to handle missing events.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>projected45</strong> \u2014 Project times from latitude 45\u00B0 when local computation fails.</li>' +
      '<li><strong>strict</strong> \u2014 Return "None" for events that cannot be astronomically computed.</li>' +
      '</ul>' +
      '<p><span class="concept-badge">Confidence</span> A 0.0\u20131.0 score indicating reliability. Standard methods score 1.0, projected/virtual score lower depending on latitude deviation.</p>' +
      '</div></div>';

    docsView.innerHTML = html;
  }

  // ═══════════════════════════════════════════════════════════
  //  13. DISAMBIGUATION
  // ═══════════════════════════════════════════════════════════

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

  // ═══════════════════════════════════════════════════════════
  //  14. STATE MANAGEMENT
  // ═══════════════════════════════════════════════════════════

  function showLoading() {
    errorState.style.display = 'none';
    calendarView.style.display = 'none';
    dayView.style.display = 'none';
    docsView.style.display = 'none';
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
    docsView.style.display = 'none';
    hideDisambiguation();
    errorMsg.textContent = msg;
    errorState.style.display = 'block';
  }

  // ═══════════════════════════════════════════════════════════
  //  15. INIT
  // ═══════════════════════════════════════════════════════════

  // Fetch cities for autocomplete
  api.cities()
    .then(function (data) { state.cities = data; })
    .catch(function () {});

  // Set up router and navigate
  initRouter();
  state.currentRoute = getRoute();
  setActiveNav(state.currentRoute);
  navigateToRoute(state.currentRoute);

  // Coord copy delegation
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
