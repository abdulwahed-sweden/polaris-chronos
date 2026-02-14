// Polaris Chronos — Dashboard App

(function () {
  'use strict';

  const cityInput = document.getElementById('city-input');
  const dateInput = document.getElementById('date-input');
  const strategyInput = document.getElementById('strategy-input');
  const goBtn = document.getElementById('go-btn');
  const errorMsg = document.getElementById('error-msg');
  const results = document.getElementById('results');
  const autocompleteList = document.getElementById('autocomplete-list');

  let cities = [];
  let selectedIndex = -1;

  // Set default date to today
  const today = new Date().toISOString().split('T')[0];
  dateInput.value = today;

  // Load cities for autocomplete
  fetch('/api/cities')
    .then(r => r.json())
    .then(data => { cities = data; })
    .catch(() => { /* ignore — autocomplete just won't work */ });

  // ── Autocomplete ───────────────────────────────────────────────

  cityInput.addEventListener('input', function () {
    const val = this.value.trim().toLowerCase();
    selectedIndex = -1;

    if (val.length < 1) {
      closeAutocomplete();
      return;
    }

    const matches = cities.filter(c =>
      c.name.toLowerCase().includes(val)
    ).slice(0, 8);

    if (matches.length === 0) {
      closeAutocomplete();
      return;
    }

    autocompleteList.innerHTML = '';
    matches.forEach((city, i) => {
      const item = document.createElement('div');
      item.className = 'autocomplete-item';
      item.dataset.index = i;
      item.innerHTML =
        '<span>' + capitalize(city.name) + '</span>' +
        '<span class="country-code">' + city.country + '</span>';
      item.addEventListener('mousedown', function (e) {
        e.preventDefault();
        cityInput.value = capitalize(city.name);
        closeAutocomplete();
        fetchTimes();
      });
      autocompleteList.appendChild(item);
    });

    autocompleteList.classList.add('active');
  });

  cityInput.addEventListener('keydown', function (e) {
    const items = autocompleteList.querySelectorAll('.autocomplete-item');
    if (!items.length) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      highlightItem(items);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      highlightItem(items);
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      e.preventDefault();
      items[selectedIndex].dispatchEvent(new Event('mousedown'));
    }
  });

  cityInput.addEventListener('blur', function () {
    setTimeout(closeAutocomplete, 150);
  });

  function closeAutocomplete() {
    autocompleteList.classList.remove('active');
    autocompleteList.innerHTML = '';
    selectedIndex = -1;
  }

  function highlightItem(items) {
    items.forEach(el => el.classList.remove('selected'));
    if (selectedIndex >= 0 && selectedIndex < items.length) {
      items[selectedIndex].classList.add('selected');
    }
  }

  // ── Fetch & Render ─────────────────────────────────────────────

  goBtn.addEventListener('click', fetchTimes);
  cityInput.addEventListener('keydown', function (e) {
    if (e.key === 'Enter' && selectedIndex < 0) fetchTimes();
  });

  function fetchTimes() {
    const city = cityInput.value.trim();
    if (!city) {
      showError('Please enter a city name.');
      return;
    }

    hideError();
    goBtn.disabled = true;
    goBtn.textContent = '...';

    const params = new URLSearchParams();
    params.set('city', city);
    if (dateInput.value) params.set('date', dateInput.value);
    params.set('strategy', strategyInput.value);

    fetch('/api/times?' + params.toString())
      .then(r => {
        if (!r.ok) return r.json().then(j => { throw new Error(j.error || 'Request failed'); });
        return r.json();
      })
      .then(data => {
        renderResults(data);
      })
      .catch(err => {
        showError(err.message);
        results.style.display = 'none';
      })
      .finally(() => {
        goBtn.disabled = false;
        goBtn.textContent = 'Go';
      });
  }

  function renderResults(data) {
    results.style.display = 'block';

    // Location banner
    document.getElementById('loc-name').textContent =
      capitalize(data.location.name) +
      (data.location.country_code ? ', ' + data.location.country_code : '');
    document.getElementById('loc-date').textContent = data.date;

    const stateEl = document.getElementById('day-state');
    stateEl.textContent = data.state;
    stateEl.className = 'day-state ' + data.state.toLowerCase().replace(/\s+/g, '');

    // Prayer table
    const tbody = document.getElementById('prayer-tbody');
    tbody.innerHTML = '';
    const prayers = ['fajr', 'sunrise', 'dhuhr', 'asr', 'maghrib', 'isha'];
    prayers.forEach(name => {
      const ev = data.events[name];
      const tr = document.createElement('tr');

      // Prayer name
      const tdName = document.createElement('td');
      tdName.textContent = capitalize(name);
      tr.appendChild(tdName);

      // Time
      const tdTime = document.createElement('td');
      if (ev.time) {
        tdTime.textContent = formatTime(ev.time) + (ev.next_day ? ' +1' : '');
      } else {
        tdTime.textContent = '---';
        tdTime.style.color = 'var(--text-dim)';
      }
      tr.appendChild(tdTime);

      // Method
      const tdMethod = document.createElement('td');
      const methodClass = ev.method.toLowerCase();
      tdMethod.innerHTML =
        '<span class="method-badge ' + methodClass + '">' +
        '<span class="method-dot ' + methodClass + '"></span>' +
        ev.method +
        '</span>';
      tr.appendChild(tdMethod);

      // Confidence
      const tdConf = document.createElement('td');
      const confPct = Math.round(ev.confidence * 100);
      const confClass = ev.confidence >= 0.9 ? 'high' : ev.confidence >= 0.6 ? 'medium' : 'low';
      tdConf.innerHTML =
        '<div class="confidence-cell">' +
        '<div class="confidence-bar"><div class="confidence-fill ' + confClass + '" style="width:' + confPct + '%"></div></div>' +
        '<span class="confidence-value">' + ev.confidence.toFixed(1) + '</span>' +
        '</div>';
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

  // ── Helpers ────────────────────────────────────────────────────

  function capitalize(s) {
    return s.replace(/\b\w/g, c => c.toUpperCase());
  }

  function formatTime(t) {
    // Input: "HH:MM:SS" or "HH:MM" — show HH:MM
    return t.substring(0, 5);
  }

  function showError(msg) {
    errorMsg.textContent = msg;
    errorMsg.style.display = 'block';
  }

  function hideError() {
    errorMsg.style.display = 'none';
  }
})();
