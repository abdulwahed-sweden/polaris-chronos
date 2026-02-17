(function () {
  'use strict';

  // ═══════════════════════════════════════════════════════════
  //  1. CONFIG & CONSTANTS
  // ═══════════════════════════════════════════════════════════

  var DEFAULT_CITY = 'Troms\u00f8';

  var PRAYER_NAMES = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];

  var METHOD_ICONS = {
    'Standard': '\u{1F7E2}',
    'Projected': '\u{1F7E0}',
    'Virtual': '\u{1F7E3}',
    'None': '\u2B1C'
  };

  var METHOD_COLORS = {
    'Standard': '#059669',
    'Projected': '#d97706',
    'Virtual': '#7C3AED',
    'None': '#9CA3AF'
  };

  var HIJRI_MONTH_NAMES = [
    '', 'Muharram', 'Safar', 'Rabi al-Awwal', 'Rabi al-Thani',
    'Jumada al-Ula', 'Jumada al-Thani', 'Rajab', 'Sha\'ban',
    'Ramadan', 'Shawwal', 'Dhu al-Qi\'dah', 'Dhu al-Hijjah'
  ];

  // Content sensitivity filter
  var FILTERED_TERMS = ['israel', 'israeli'];

  // ═══════════════════════════════════════════════════════════
  //  2. STATE
  // ═══════════════════════════════════════════════════════════

  var state = {
    cities: [],
    currentLoc: null,
    currentRoute: 'calendar',
    calendarData: null,
    allDaysCache: null,
    dayData: null,
    ramadanMeta: null,
    hijriData: null,
    viewMode: 'month',
    hijriMode: true,
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
  var gpsBtn = $('gps-btn');
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
  //  4. CONTENT FILTER
  // ═══════════════════════════════════════════════════════════

  function filterSensitiveText(text) {
    if (!text) return text;
    var result = text;
    FILTERED_TERMS.forEach(function (term) {
      var re = new RegExp('\\b' + term + '\\b', 'gi');
      result = result.replace(re, '');
    });
    // Remove Hebrew characters (Unicode range U+0590-U+05FF)
    result = result.replace(/[\u0590-\u05FF]+/g, '');
    // Clean up double commas, leading/trailing commas, extra spaces
    result = result.replace(/,\s*,/g, ',').replace(/^\s*,\s*/, '').replace(/\s*,\s*$/, '');
    result = result.replace(/\s{2,}/g, ' ').trim();
    return result;
  }

  function filterLocation(loc) {
    if (!loc) return loc;
    var filtered = Object.assign({}, loc);
    if (filtered.name) filtered.name = filterSensitiveText(filtered.name);
    if (filtered.country) filtered.country = filterSensitiveText(filtered.country);
    if (filtered.display_name) filtered.display_name = filterSensitiveText(filtered.display_name);
    return filtered;
  }

  // ═══════════════════════════════════════════════════════════
  //  5. ROUTER
  // ═══════════════════════════════════════════════════════════

  function getRoute() {
    var path = window.location.pathname;
    if (path === '/docs') return 'docs';
    if (path === '/day') return 'day';
    return 'calendar';
  }

  function setActiveNav(route) {
    document.querySelectorAll('.header-nav .nav-link').forEach(function (link) {
      link.classList.remove('active');
      if (link.dataset.route === route) {
        link.classList.add('active');
      }
    });
  }

  function initRouter() {
    document.querySelectorAll('.header-nav .nav-link[data-route]').forEach(function (link) {
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
  //  6. API LAYER
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
    hijri: function (params) {
      var q = 'lat=' + params.lat + '&lon=' + params.lon +
        '&tz=' + encodeURIComponent(params.tz);
      if (params.hijri_year) q += '&hijri_year=' + params.hijri_year;
      return fetch('/api/hijri?' + q, { cache: 'no-store' })
        .then(function (r) {
          if (!r.ok) return r.json().then(function (j) { throw new Error(j.error || 'Failed'); });
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
  //  7. UTILITIES
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
    var name = capitalize(filterSensitiveText(loc.name));
    if (loc.country) {
      var country = filterSensitiveText(loc.country);
      if (country) name += ', ' + country;
    } else if (loc.country_code) {
      name += ', ' + loc.country_code;
    }
    return name;
  }

  function formatHijriDate(h) {
    if (!h) return '';
    var monthName = HIJRI_MONTH_NAMES[h.month] || '';
    return h.day + ' ' + monthName + ' ' + h.year + ' AH';
  }

  // ═══════════════════════════════════════════════════════════
  //  8. AUTOCOMPLETE
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
        var cityCountry = filterSensitiveText(city.country);
        var item = document.createElement('div');
        item.className = 'autocomplete-item';
        item.innerHTML = '<span>' + escapeHtml(capitalize(city.name)) + '</span>' +
          '<span class="country-badge">' + escapeHtml(cityCountry) + '</span>';
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
  //  9. GPS AUTO-DETECT
  // ═══════════════════════════════════════════════════════════

  function initGPS() {
    if (!gpsBtn) return;
    gpsBtn.addEventListener('click', function () {
      if (!navigator.geolocation) {
        showError('Geolocation is not supported by your browser.');
        return;
      }
      gpsBtn.classList.add('locating');
      navigator.geolocation.getCurrentPosition(
        function (pos) {
          gpsBtn.classList.remove('locating');
          var lat = pos.coords.latitude;
          var lon = pos.coords.longitude;
          cityInput.value = formatCoords(lat, lon);
          // Use coords directly
          showLoading();
          var loc = {
            name: formatCoords(lat, lon),
            lat: lat,
            lon: lon,
            tz: Intl.DateTimeFormat().resolvedOptions().timeZone,
            tz_label: Intl.DateTimeFormat().resolvedOptions().timeZone + ' (Local Time)',
            formatted_coords: formatCoords(lat, lon),
            source: 'GPS',
            confidence: 1.0
          };
          fetchRamadanMonth(loc);
        },
        function (err) {
          gpsBtn.classList.remove('locating');
          showError('Location access denied. Please search manually.');
        },
        { timeout: 10000, enableHighAccuracy: false }
      );
    });
  }

  // ═══════════════════════════════════════════════════════════
  //  10. NOW DASHBOARD
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
  //  11. SHARE FUNCTIONALITY
  // ═══════════════════════════════════════════════════════════

  function generateShareText() {
    if (!state.calendarData || !state.calendarData.length || !state.currentLoc) return null;

    var todayStr = isoDate(new Date());
    var todayData = null;
    for (var i = 0; i < state.calendarData.length; i++) {
      if (state.calendarData[i].date === todayStr) {
        todayData = state.calendarData[i];
        break;
      }
    }

    if (!todayData) {
      todayData = state.calendarData[0];
    }

    var loc = displayName(state.currentLoc);
    var fajr = todayData.data.events.fajr;
    var maghrib = todayData.data.events.maghrib;
    var meta = state.ramadanMeta;

    var text = 'Prayer Times - ' + loc + '\n';
    if (meta && todayData.ramadanDay) {
      text += 'Ramadan ' + todayData.ramadanDay + ' / ' + meta.hijriYear + ' AH\n';
    }
    text += todayData.date + '\n\n';
    if (fajr && fajr.time) text += 'Fajr (Suhoor ends): ' + fmtTime(fajr.time) + '\n';
    if (maghrib && maghrib.time) text += 'Maghrib (Iftar): ' + fmtTime(maghrib.time) + '\n';
    text += '\nvia Polaris Chronos';

    return text;
  }

  function handleShare(btn) {
    var text = generateShareText();
    if (!text) return;

    if (navigator.share) {
      navigator.share({ text: text }).catch(function () {});
    } else if (navigator.clipboard) {
      navigator.clipboard.writeText(text).then(function () {
        btn.classList.add('copied');
        var orig = btn.innerHTML;
        btn.textContent = 'Copied!';
        setTimeout(function () {
          btn.innerHTML = orig;
          btn.classList.remove('copied');
        }, 2000);
      });
    }
  }

  // ═══════════════════════════════════════════════════════════
  //  12. CALENDAR VIEW
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

    // View mode tabs
    document.querySelectorAll('.view-tab').forEach(function (tab) {
      tab.addEventListener('click', function () {
        document.querySelectorAll('.view-tab').forEach(function (t) { t.classList.remove('active'); });
        tab.classList.add('active');
        state.viewMode = tab.dataset.view;
        if (state.calendarData && state.currentLoc) {
          renderCalendar(state.currentLoc, state.calendarData);
        }
      });
    });

    // Hijri toggle
    var hijriToggle = $('hijri-mode');
    if (hijriToggle) {
      hijriToggle.checked = state.hijriMode;
      hijriToggle.addEventListener('change', function () {
        state.hijriMode = this.checked;
        if (state.currentLoc) {
          if (state.hijriMode) {
            fetchRamadanMonth(state.currentLoc);
          } else {
            fetchGregorianMonth(state.currentLoc);
          }
        }
      });
    }

    // Share button
    var shareBtn = $('share-btn');
    if (shareBtn) {
      shareBtn.addEventListener('click', function () { handleShare(this); });
    }

    loadCalendarForCity(city);
  }

  function loadCalendarForCity(city) {
    showLoading();
    hideDisambiguation();

    api.resolve(city)
      .then(function (loc) {
        loc = filterLocation(loc);
        if (state.hijriMode) {
          fetchRamadanMonth(loc);
        } else {
          fetchRamadanMonth(loc);
        }
      })
      .catch(function (err) {
        if (err.message === 'DISAMBIGUATE') {
          showDisambiguation(err.data, function (loc) {
            loc = filterLocation(loc);
            fetchRamadanMonth(loc);
          });
        } else {
          showError(err.message);
        }
      });
  }

  function fetchRamadanMonth(loc) {
    api.hijri({ lat: loc.lat, lon: loc.lon, tz: loc.tz })
      .then(function (hijriData) {
        state.hijriData = hijriData;
        var meta = hijriData.ramadan;
        var startDate = new Date(meta.start + 'T12:00:00');
        var endDate = new Date(meta.end + 'T12:00:00');

        state.ramadanMeta = {
          startDate: startDate,
          endDate: endDate,
          days: meta.days,
          hijriYear: meta.hijri_year,
          conjunction: meta.conjunction,
          visibility: meta.visibility,
          shawwalStart: meta.shawwal_start
        };

        var startMonth = startDate.getMonth() + 1;
        var startYear = startDate.getFullYear();
        var endMonth = endDate.getMonth() + 1;
        var endYear = endDate.getFullYear();

        var monthPromises = [];
        var y = startYear, m = startMonth;
        while (y < endYear || (y === endYear && m <= endMonth)) {
          monthPromises.push(api.month({ lat: loc.lat, lon: loc.lon, tz: loc.tz, year: y, month: m }));
          m++;
          if (m > 12) { m = 1; y++; }
        }

        return Promise.all(monthPromises);
      })
      .then(function (results) {
        var allDays = {};
        results.forEach(function (monthData) {
          monthData.forEach(function (d) { allDays[d.date] = d; });
        });

        state.allDaysCache = allDays;

        var ramadanData = [];
        var meta = state.ramadanMeta;
        for (var i = 0; i < meta.days; i++) {
          var date = new Date(meta.startDate);
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

  function fetchGregorianMonth(loc) {
    var now = new Date();
    var year = now.getFullYear();
    var month = now.getMonth() + 1;

    showLoading();
    api.month({ lat: loc.lat, lon: loc.lon, tz: loc.tz, year: year, month: month })
      .then(function (monthData) {
        var calData = monthData.map(function (d, i) {
          return { date: d.date, ramadanDay: i + 1, data: d };
        });
        state.calendarData = calData;
        renderCalendar(loc, calData);
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

    // City dashboard
    $('cal-loc-name').textContent = displayName(loc);
    $('cal-loc-tz').textContent = loc.tz_label || loc.tz;
    $('cal-loc-coords').textContent = loc.formatted_coords || formatCoords(loc.lat, loc.lon);

    // Dual date display
    var hijriDisplay = $('hijri-date-display');
    var gregDisplay = $('gregorian-date-display');
    if (state.hijriData && state.hijriData.hijri_date) {
      hijriDisplay.textContent = formatHijriDate(state.hijriData.hijri_date);
    }
    gregDisplay.textContent = new Date().toLocaleDateString('en', {
      weekday: 'short', year: 'numeric', month: 'long', day: 'numeric'
    });

    // Ramadan header
    var meta = state.ramadanMeta;
    $('ramadan-title').textContent = state.hijriMode ?
      'Ramadan ' + (meta ? meta.hijriYear + ' AH' : '') :
      new Date().toLocaleDateString('en', { month: 'long', year: 'numeric' });
    if (meta && state.hijriMode) {
      $('ramadan-dates').textContent = displayDate(meta.startDate) + ' \u2014 ' + displayDate(meta.endDate);
    } else {
      $('ramadan-dates').textContent = '';
    }

    // Determine which days to show based on view mode
    var visibleDays = days;
    if (state.viewMode === 'week') {
      visibleDays = getWeekDays(days);
    } else if (state.viewMode === 'day') {
      visibleDays = getTodayDay(days);
    }

    // Calendar table
    var tbody = $('calendar-tbody');
    tbody.innerHTML = '';
    var todayStr = isoDate(new Date());

    visibleDays.forEach(function (day) {
      var tr = document.createElement('tr');
      if (day.date === todayStr) tr.className = 'today-row';

      var dateObj = new Date(day.date + 'T12:00:00');
      var dayName = dateObj.toLocaleDateString('en', { weekday: 'short' });
      var monthDay = dateObj.toLocaleDateString('en', { month: 'short', day: 'numeric' });

      // Day number
      var tdDay = document.createElement('td');
      tdDay.className = 'cal-day-num';
      tdDay.textContent = state.hijriMode ? day.ramadanDay : dateObj.getDate();
      tr.appendChild(tdDay);

      // Date
      var tdDate = document.createElement('td');
      tdDate.className = 'cal-date';
      tdDate.textContent = dayName + ', ' + monthDay;
      tr.appendChild(tdDate);

      // Prayer times
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
            badge.style.fontSize = '0.6rem';
            badge.style.padding = '0.05rem 0.3rem';
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

    updateNowDashboard(days);
  }

  function getWeekDays(days) {
    var todayStr = isoDate(new Date());
    var todayIdx = -1;
    for (var i = 0; i < days.length; i++) {
      if (days[i].date === todayStr) { todayIdx = i; break; }
    }
    if (todayIdx === -1) todayIdx = 0;

    var start = Math.max(0, todayIdx);
    var end = Math.min(days.length, start + 7);
    return days.slice(start, end);
  }

  function getTodayDay(days) {
    var todayStr = isoDate(new Date());
    for (var i = 0; i < days.length; i++) {
      if (days[i].date === todayStr) return [days[i]];
    }
    return days.length > 0 ? [days[0]] : [];
  }

  // ═══════════════════════════════════════════════════════════
  //  13. DAY VIEW
  // ═══════════════════════════════════════════════════════════

  function initDayView() {
    calendarView.style.display = 'none';
    docsView.style.display = 'none';

    var params = new URLSearchParams(window.location.search);
    var date = params.get('date');
    var lat = parseFloat(params.get('lat'));
    var lon = parseFloat(params.get('lon'));
    var tz = params.get('tz');
    var name = filterSensitiveText(params.get('name') || '');
    var country = filterSensitiveText(params.get('country') || '');
    var cc = params.get('cc') || '';

    if (!date || isNaN(lat) || isNaN(lon) || !tz) {
      showError('Missing parameters. Use the calendar to select a day.');
      return;
    }

    var locDisplayName = capitalize(name);
    if (country) locDisplayName += ', ' + country;
    else if (cc) locDisplayName += ', ' + cc;

    $('day-loc-name').textContent = locDisplayName;
    $('day-loc-tz').textContent = tz + ' (Local Time)';
    $('day-loc-coords').textContent = formatCoords(lat, lon);

    var dateObj = new Date(date + 'T12:00:00');
    var meta = state.ramadanMeta;
    var ramadanDay = meta ? Math.round((dateObj - meta.startDate) / 86400000) + 1 : -1;
    var dayLabel = (meta && ramadanDay >= 1 && ramadanDay <= meta.days) ?
      'Ramadan ' + ramadanDay : dateObj.toLocaleDateString('en', { month: 'long', day: 'numeric', year: 'numeric' });

    $('day-title').textContent = dayLabel + ' \u2014 ' + locDisplayName;
    $('day-subtitle').textContent =
      dateObj.toLocaleDateString('en', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });

    cityInput.value = capitalize(name);

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

    // Share button
    var dayShareBtn = $('day-share-btn');
    if (dayShareBtn) {
      dayShareBtn.addEventListener('click', function () { handleShare(this); });
    }

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
  //  14. HORIZON DIAL SVG
  // ═══════════════════════════════════════════════════════════

  function renderHorizonDial(data) {
    var container = $('horizon-dial-container');
    var dial = $('horizon-dial');
    dial.innerHTML = '';

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
      container.style.display = 'block';
      dial.innerHTML = '<div style="text-align:center;padding:2rem;color:#6b7280;">' +
        'Insufficient solar data for horizon visualization (polar conditions).</div>';
      return;
    }

    container.style.display = 'block';
    prayerTimes.sort(function (a, b) { return a.hours - b.hours; });

    var W = 800, H = 420;
    var PAD_L = 60, PAD_R = 40, PAD_T = 55, PAD_B = 70;
    var chartW = W - PAD_L - PAD_R;
    var chartH = H - PAD_T - PAD_B;
    var horizonY = PAD_T + chartH * 0.55;

    var firstHour = prayerTimes[0].hours;
    var lastHour = prayerTimes[prayerTimes.length - 1].hours;
    var timeSpan = lastHour - firstHour;
    if (timeSpan <= 0) timeSpan = 1;
    var timePad = timeSpan * 0.06;
    var tMin = firstHour - timePad;
    var tMax = lastHour + timePad;
    var tRange = tMax - tMin;

    function timeToX(h) { return PAD_L + ((h - tMin) / tRange) * chartW; }

    var sunriseIdx = -1, sunsetIdx = -1;
    for (var i = 0; i < prayerTimes.length; i++) {
      if (prayerTimes[i].name === 'sunrise') sunriseIdx = i;
      if (prayerTimes[i].name === 'maghrib') sunsetIdx = i;
    }
    var riseH = sunriseIdx >= 0 ? prayerTimes[sunriseIdx].hours : firstHour;
    var setH = sunsetIdx >= 0 ? prayerTimes[sunsetIdx].hours : lastHour;
    var noonH = (riseH + setH) / 2;
    var daySpan = setH - riseH;
    if (daySpan <= 0) daySpan = 1;

    function altitudeY(h) {
      var fromNoon = (h - noonH) / (daySpan / 2);
      var alt = 1 - fromNoon * fromNoon;
      var aboveSpace = horizonY - PAD_T - 15;
      var belowSpace = 50;
      if (alt >= 0) {
        return horizonY - alt * aboveSpace;
      } else {
        return horizonY - alt * belowSpace;
      }
    }

    var ns = 'http://www.w3.org/2000/svg';
    var svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
    svg.setAttribute('preserveAspectRatio', 'xMidYMid meet');

    var skyRect = document.createElementNS(ns, 'rect');
    skyRect.setAttribute('x', PAD_L); skyRect.setAttribute('y', PAD_T);
    skyRect.setAttribute('width', chartW); skyRect.setAttribute('height', horizonY - PAD_T);
    skyRect.setAttribute('class', 'dial-sky-fill');
    svg.appendChild(skyRect);

    var gndRect = document.createElementNS(ns, 'rect');
    gndRect.setAttribute('x', PAD_L); gndRect.setAttribute('y', horizonY);
    gndRect.setAttribute('width', chartW); gndRect.setAttribute('height', PAD_T + chartH - horizonY);
    gndRect.setAttribute('class', 'dial-ground-fill');
    svg.appendChild(gndRect);

    var skyLabel = document.createElementNS(ns, 'text');
    skyLabel.setAttribute('x', PAD_L + 8); skyLabel.setAttribute('y', PAD_T + 18);
    skyLabel.setAttribute('class', 'dial-zone-label');
    skyLabel.textContent = 'ABOVE HORIZON';
    svg.appendChild(skyLabel);

    var gndLabel = document.createElementNS(ns, 'text');
    gndLabel.setAttribute('x', PAD_L + 8); gndLabel.setAttribute('y', horizonY + 18);
    gndLabel.setAttribute('class', 'dial-zone-label');
    gndLabel.textContent = 'BELOW HORIZON';
    svg.appendChild(gndLabel);

    var hLine = document.createElementNS(ns, 'line');
    hLine.setAttribute('x1', PAD_L); hLine.setAttribute('y1', horizonY);
    hLine.setAttribute('x2', PAD_L + chartW); hLine.setAttribute('y2', horizonY);
    hLine.setAttribute('class', 'dial-horizon');
    svg.appendChild(hLine);

    var hLabel = document.createElementNS(ns, 'text');
    hLabel.setAttribute('x', PAD_L - 8); hLabel.setAttribute('y', horizonY + 4);
    hLabel.setAttribute('text-anchor', 'end');
    hLabel.setAttribute('class', 'dial-horizon-label');
    hLabel.textContent = '0\u00B0';
    svg.appendChild(hLabel);

    var curvePoints = [];
    var steps = 80;
    for (var s = 0; s <= steps; s++) {
      var t = tMin + (s / steps) * tRange;
      var cx = timeToX(t);
      var cy = altitudeY(t);
      cy = Math.max(PAD_T, Math.min(PAD_T + chartH, cy));
      curvePoints.push({ x: cx, y: cy });
    }

    var areaPath = 'M ' + curvePoints[0].x.toFixed(1) + ' ' + horizonY;
    for (var k = 0; k < curvePoints.length; k++) {
      areaPath += ' L ' + curvePoints[k].x.toFixed(1) + ' ' + curvePoints[k].y.toFixed(1);
    }
    areaPath += ' L ' + curvePoints[curvePoints.length - 1].x.toFixed(1) + ' ' + horizonY + ' Z';
    var area = document.createElementNS(ns, 'path');
    area.setAttribute('d', areaPath);
    area.setAttribute('class', 'dial-sun-area');
    svg.appendChild(area);

    var linePath = 'M ' + curvePoints[0].x.toFixed(1) + ' ' + curvePoints[0].y.toFixed(1);
    for (var k2 = 1; k2 < curvePoints.length; k2++) {
      linePath += ' L ' + curvePoints[k2].x.toFixed(1) + ' ' + curvePoints[k2].y.toFixed(1);
    }
    var sunLine = document.createElementNS(ns, 'path');
    sunLine.setAttribute('d', linePath);
    sunLine.setAttribute('class', 'dial-sun-path');
    svg.appendChild(sunLine);

    prayerTimes.forEach(function (pt) {
      var mx = timeToX(pt.hours);
      var my = altitudeY(pt.hours);
      my = Math.max(PAD_T + 5, Math.min(PAD_T + chartH - 5, my));
      var color = METHOD_COLORS[pt.method] || '#9CA3AF';

      var vLine = document.createElementNS(ns, 'line');
      vLine.setAttribute('x1', mx); vLine.setAttribute('y1', H - PAD_B + 5);
      vLine.setAttribute('x2', mx); vLine.setAttribute('y2', my);
      vLine.setAttribute('class', 'dial-marker-line');
      svg.appendChild(vLine);

      var dot = document.createElementNS(ns, 'circle');
      dot.setAttribute('cx', mx); dot.setAttribute('cy', my);
      dot.setAttribute('r', 7);
      dot.setAttribute('fill', color);
      dot.setAttribute('class', 'dial-marker-dot');
      svg.appendChild(dot);

      var nameY = H - PAD_B + 22;
      var nameLabel = document.createElementNS(ns, 'text');
      nameLabel.setAttribute('x', mx); nameLabel.setAttribute('y', nameY);
      nameLabel.setAttribute('text-anchor', 'middle');
      nameLabel.setAttribute('class', 'dial-marker-label');
      nameLabel.textContent = capitalize(pt.name);
      svg.appendChild(nameLabel);

      var hh = Math.floor(pt.hours);
      var mm = Math.round((pt.hours - hh) * 60);
      var timeStr = pad2(hh) + ':' + pad2(mm);

      var timeLabel = document.createElementNS(ns, 'text');
      timeLabel.setAttribute('x', mx); timeLabel.setAttribute('y', nameY + 17);
      timeLabel.setAttribute('text-anchor', 'middle');
      timeLabel.setAttribute('class', 'dial-marker-time');
      timeLabel.textContent = timeStr;
      svg.appendChild(timeLabel);
    });

    var todayStr = isoDate(new Date());
    if (data.date === todayStr) {
      var now = new Date();
      var nowHours = now.getHours() + now.getMinutes() / 60;
      if (nowHours >= tMin && nowHours <= tMax) {
        var sx = timeToX(nowHours);
        var sy = altitudeY(nowHours);
        sy = Math.max(PAD_T + 5, Math.min(PAD_T + chartH - 5, sy));

        var sunCircle = document.createElementNS(ns, 'circle');
        sunCircle.setAttribute('cx', sx);
        sunCircle.setAttribute('cy', sy);
        sunCircle.setAttribute('r', 10);
        sunCircle.setAttribute('class', 'dial-sun dial-sun-pulse');
        svg.appendChild(sunCircle);
      }
    }

    dial.appendChild(svg);
  }

  // ═══════════════════════════════════════════════════════════
  //  15. DOCS VIEW — Polaris for Developers
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
        path: '/api/hijri',
        desc: 'Get Hijri calendar data including Ramadan start/end dates computed via astronomical crescent visibility (Odeh 2004 criterion).',
        params: [
          { name: 'lat', type: 'number', required: true, desc: 'Latitude (-90 to 90)' },
          { name: 'lon', type: 'number', required: true, desc: 'Longitude (-180 to 180)' },
          { name: 'tz', type: 'string', required: true, desc: 'IANA timezone' },
          { name: 'hijri_year', type: 'number', required: false, desc: 'Hijri year (defaults to current)' }
        ],
        curl: "curl 'http://localhost:3000/api/hijri?lat=21.42&lon=39.83&tz=Asia/Riyadh'",
        js: "const res = await fetch('/api/hijri?lat=21.42&lon=39.83&tz=Asia/Riyadh');\nconst data = await res.json();\nconsole.log('Ramadan starts:', data.ramadan.start);"
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
      '<h1>Polaris for Developers</h1>' +
      '<p>A RESTful API for computing prayer times at any location on Earth, including polar regions with adaptive compensation and astronomical Hijri calendar.</p>' +
      '</div>';

    endpoints.forEach(function (ep) {
      html += '<div class="polaris-card docs-endpoint">' +
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

    html += '<div class="docs-concepts">' +
      '<h3>Concepts</h3>' +
      '<div class="polaris-card">' +
      '<p style="margin-bottom:1rem;"><span class="concept-badge">DayState</span> Describes the solar condition for a given day at the location.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>Normal</strong> \u2014 Standard sunrise/sunset cycle. All times are astronomically computed.</li>' +
      '<li><strong>WhiteNight</strong> \u2014 Sun dips below horizon but not far enough for twilight-based events.</li>' +
      '<li><strong>PolarDay</strong> \u2014 Sun never sets (midnight sun). Sunset/twilight events require compensation.</li>' +
      '<li><strong>PolarNight</strong> \u2014 Sun never rises. Sunrise/daytime events require compensation.</li>' +
      '</ul>' +
      '<p style="margin-bottom:1rem;"><span class="concept-badge">EventMethod</span> How each prayer time was determined.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>Standard</strong> \u2014 Real astronomical calculation from sun position.</li>' +
      '<li><strong>Projected</strong> \u2014 Interpolated from the nearest latitude (45\u00B0) where the event occurs naturally.</li>' +
      '<li><strong>Virtual</strong> \u2014 Computed using virtual horizon techniques for extreme conditions.</li>' +
      '<li><strong>None</strong> \u2014 Event cannot be computed (e.g. no sunset in polar day with strict strategy).</li>' +
      '</ul>' +
      '<p style="margin-bottom:1rem;"><span class="concept-badge">CrescentVisibility</span> How Ramadan start is determined.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>Zone A</strong> \u2014 Crescent visible to naked eye.</li>' +
      '<li><strong>Zone B</strong> \u2014 May need optical aid, sometimes naked eye.</li>' +
      '<li><strong>Zone C</strong> \u2014 Requires optical aid.</li>' +
      '<li><strong>Zone D</strong> \u2014 Not visible (below Odeh criterion threshold).</li>' +
      '</ul>' +
      '<p style="margin-bottom:1rem;"><span class="concept-badge">GapStrategy</span> How to handle missing events.</p>' +
      '<ul style="color:var(--text-secondary);margin-left:1.5rem;margin-bottom:1rem;">' +
      '<li><strong>projected45</strong> \u2014 Project times from latitude 45\u00B0 when local computation fails.</li>' +
      '<li><strong>strict</strong> \u2014 Return "None" for events that cannot be astronomically computed.</li>' +
      '</ul>' +
      '<p><span class="concept-badge">Confidence</span> A 0.0\u20131.0 score indicating reliability. Standard methods score 1.0, projected/virtual score lower depending on latitude deviation.</p>' +
      '</div></div>';

    docsView.innerHTML = html;
  }

  // ═══════════════════════════════════════════════════════════
  //  16. DISAMBIGUATION
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
      var optCountry = filterSensitiveText(opt.country);
      html += '<button class="disambig-btn" data-cc="' + escapeHtml(opt.country_code) + '" ' +
        'data-lat="' + opt.lat + '" data-lon="' + opt.lon + '" data-tz="' + escapeHtml(opt.tz) + '" ' +
        'data-name="' + escapeHtml(opt.name) + '">' +
        '<span class="disambig-name">' + escapeHtml(opt.name) + '</span>' +
        '<span class="disambig-detail">' + escapeHtml(optCountry) + ' \u00B7 ' + escapeHtml(opt.tz) + '</span>' +
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
        onSelect(filterLocation(loc));
      });
    });
  }

  function hideDisambiguation() {
    var el = document.getElementById('disambiguation');
    if (el) el.style.display = 'none';
  }

  // ═══════════════════════════════════════════════════════════
  //  17. STATE MANAGEMENT
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
  //  18. INIT
  // ═══════════════════════════════════════════════════════════

  // Fetch cities for autocomplete
  api.cities()
    .then(function (data) { state.cities = data; })
    .catch(function () {});

  // Set up router and navigate
  initRouter();
  initGPS();
  state.currentRoute = getRoute();
  setActiveNav(state.currentRoute);
  navigateToRoute(state.currentRoute);

  // Coord copy delegation
  document.addEventListener('click', function (e) {
    if (e.target.classList.contains('city-coords')) {
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
