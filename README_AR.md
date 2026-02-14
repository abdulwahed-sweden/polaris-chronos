<h1 align="center">بولاريس</h1>

<p align="center">
  <strong>محرك مواقيت الصلاة العالمي</strong>
</p>

<p align="center">
  محرك فلكي عالي الدقة مكتوب بلغة Rust<br>
  يحسب أوقات الصلاة لأي مكان على وجه الأرض — بما في ذلك المناطق القطبية
</p>

<p align="center">
  <a href="README.md">English</a>&nbsp;&nbsp;·&nbsp;&nbsp;<a href="README_AR.md">العربية</a>
</p>

<br>

---

<div dir="rtl">

<br>

<h2>المشكلة</h2>

<p>
تطبيقات المواقيت التقليدية تعتمد على فكرة واحدة بسيطة:
</p>

<blockquote>
المغرب = لحظة غروب الشمس
</blockquote>

<p>
لكن في أماكن مثل شمال النرويج والسويد وألاسكا، هناك ظواهر لا تتعامل معها هذه التطبيقات:
</p>

<br>

<table>
  <thead>
    <tr>
      <th align="right">الظاهرة</th>
      <th align="right">ماذا يحدث</th>
      <th align="right">النتيجة في التطبيقات التقليدية</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="right"><strong>شمس منتصف الليل</strong></td>
      <td align="right">الشمس لا تغرب لأسابيع</td>
      <td align="right">لا يوجد مغرب ولا عشاء — خطأ أو فراغ</td>
    </tr>
    <tr>
      <td align="right"><strong>الليل القطبي</strong></td>
      <td align="right">الشمس لا تشرق لأسابيع</td>
      <td align="right">لا يوجد فجر ولا شروق — خطأ أو فراغ</td>
    </tr>
  </tbody>
</table>

<br>

<p>
بولاريس لا يتوقف عند هذه الحالات. <strong>يحلّها.</strong>
</p>

<br>

---

<br>

<h2>الفكرة الأساسية</h2>

<p>
بولاريس لا ينظر إلى الشمس كجسم "مرئي أو غير مرئي" فقط.
</p>

<p>
بل يتعامل معها كـ <strong>حركة زاوية مستمرة</strong> — موجة رياضية لا تتوقف حتى لو كانت الشمس فوق الأفق أو تحته طوال اليوم.
</p>

<br>

<p>هذا يعني:</p>

<ul>
  <li>حتى لو الشمس لم تغرب — <strong>النظام يعرف أين "كان يجب" أن تغرب</strong></li>
  <li>حتى لو لم يكن هناك شفق — <strong>النظام يحسب متى "كان سيظهر" الشفق</strong></li>
</ul>

<br>

<p>
النتيجة: <strong>جدول صلاة كامل، لكل يوم، في أي مكان.</strong>
</p>

<br>

---

<br>

<h2>طرق الحساب الثلاث</h2>

<p>
بولاريس يختار الطريقة تلقائياً حسب الحالة الفلكية:
</p>

<br>

<h3>الحساب الطبيعي (Standard) — الثقة: 1.0</h3>

<p>
عندما تشرق الشمس وتغرب بشكل طبيعي.
<br>
هذا هو الحساب الفلكي المباشر — نفس المنهج المستخدم في معظم التطبيقات. لا فرق هنا.
</p>

<br>

<h3>الحساب الافتراضي (Virtual) — الثقة: 0.7</h3>

<p>
عندما لا تصل الشمس إلى الزاوية المطلوبة للفجر أو العشاء (مثلاً: لا يوجد شفق حقيقي).
</p>

<p>
يقوم النظام باشتقاق الوقت من <strong>أدنى نقطة في الموجة الشمسية</strong> — وهي النقطة التي تمثل "منتصف الليل الفلكي" حتى لو لم يكن هناك ظلام فعلي.
</p>

<br>

<h3>الإسقاط التكيّفي (Projected) — الثقة: 0.5</h3>

<p>
عندما لا يوجد شروق أو غروب إطلاقاً.
</p>

<p>يقوم النظام بـ:</p>

<ol>
  <li>الانتقال رياضياً إلى خط عرض معتدل (~45°–55°)</li>
  <li>حساب نسب اليوم هناك (طول النهار مقابل الليل)</li>
  <li>تطبيق نفس النسب على موقعك الحقيقي</li>
</ol>

<br>

<blockquote>
هذا قريب من المفهوم الفقهي المعروف <strong>"التقدير بأقرب البلاد"</strong> — لكن بولاريس يحوّله إلى نموذج رياضي دقيق يعمل تلقائياً.
</blockquote>

<br>

---

<br>

<h2>مثال حقيقي: ترومسو، النرويج — 21 يونيو 2026</h2>

<p>
<strong>شمس منتصف الليل.</strong> الشمس لم تغرب. أدنى ارتفاع لها: 3.1° فوق الأفق.
</p>

<br>

<pre dir="ltr"><code>polaris Tromso --date 2026-06-21</code></pre>

<br>

<table>
  <thead>
    <tr>
      <th align="right">الصلاة</th>
      <th align="center">الوقت</th>
      <th align="center">الطريقة</th>
      <th align="center">الثقة</th>
      <th align="right">التفسير</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="right">الفجر</td>
      <td align="center"><code>00:46</code> (+1 يوم)</td>
      <td align="center">Virtual</td>
      <td align="center">0.70</td>
      <td align="right">مشتق من أدنى نقطة في الموجة</td>
    </tr>
    <tr>
      <td align="right">الشروق</td>
      <td align="center"><code>04:07</code></td>
      <td align="center">Projected</td>
      <td align="center">0.50</td>
      <td align="right">إسقاط من خط عرض 54.7°</td>
    </tr>
    <tr>
      <td align="right">الظهر</td>
      <td align="center"><code>12:46</code></td>
      <td align="center">Standard</td>
      <td align="center">1.00</td>
      <td align="right">ذروة الشمس — حساب مباشر</td>
    </tr>
    <tr>
      <td align="right">العصر</td>
      <td align="center"><code>17:57</code></td>
      <td align="center">Standard</td>
      <td align="center">1.00</td>
      <td align="right">طول الظل — حساب مباشر</td>
    </tr>
    <tr>
      <td align="right">المغرب</td>
      <td align="center"><code>21:24</code></td>
      <td align="center">Projected</td>
      <td align="center">0.50</td>
      <td align="right">إسقاط من خط عرض 54.7°</td>
    </tr>
    <tr>
      <td align="right">العشاء</td>
      <td align="center"><code>00:46</code> (+1 يوم)</td>
      <td align="center">Virtual</td>
      <td align="center">0.70</td>
      <td align="right">مشتق من أدنى نقطة في الموجة</td>
    </tr>
  </tbody>
</table>

<br>

<h3>ماذا حدث هنا؟</h3>

<p>
الشمس بقيت فوق الأفق 24 ساعة. لا يوجد غروب حقيقي ولا شروق.
</p>

<ul>
  <li><strong>الظهر والعصر:</strong> حُسبا بشكل طبيعي — الشمس لا تزال تصل لذروتها وتلقي ظلاً</li>
  <li><strong>الشروق والمغرب:</strong> أُسقطا من خط عرض معتدل (54.7°) لأنه لا توجد لحظة عبور حقيقية للأفق</li>
  <li><strong>الفجر والعشاء:</strong> اشتُقا من الموجة لأن زاوية الشفق لم تتحقق</li>
</ul>

<br>

<p>
كل قيمة مرفقة بـ: <strong>الطريقة + درجة الثقة.</strong> لا شيء مخفي.
</p>

<br>

---

<br>

<h2>ما الفرق بين بولاريس والتطبيقات الأخرى؟</h2>

<br>

<table>
  <thead>
    <tr>
      <th align="right">المقارنة</th>
      <th align="center">التطبيقات التقليدية</th>
      <th align="center">بولاريس</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="right">يعمل في المناطق القطبية</td>
      <td align="center">يفشل أو يعطي نتائج خاطئة</td>
      <td align="center">جدول كامل دائماً</td>
    </tr>
    <tr>
      <td align="right">يوضح طريقة الحساب</td>
      <td align="center">نتيجة بدون تفسير</td>
      <td align="center">كل وقت موسوم بطريقته</td>
    </tr>
    <tr>
      <td align="right">يميز بين الحقيقي والتقديري</td>
      <td align="center">كل النتائج تبدو متساوية</td>
      <td align="center">درجة ثقة لكل وقت</td>
    </tr>
    <tr>
      <td align="right">يعتمد على الفيزياء الفلكية</td>
      <td align="center">معادلات مبسطة</td>
      <td align="center">محاكاة موقع الشمس (SPA)</td>
    </tr>
    <tr>
      <td align="right">يعمل بدون إنترنت</td>
      <td align="center">يحتاج اتصال غالباً</td>
      <td align="center">قاعدة بيانات مدمجة + ذاكرة مؤقتة</td>
    </tr>
  </tbody>
</table>

<br>

---

<br>

<h2>لماذا الثقة مهمة؟</h2>

<p>
لأن <strong>الصدق أهم من الدقة الوهمية.</strong>
</p>

<p>
عندما يعطيك تطبيق وقت المغرب في ترومسو صيفاً بدون أي تنبيه — فهو يكذب عليك. الشمس لم تغرب أصلاً.
</p>

<p>
بولاريس يقول لك بوضوح:
</p>

<blockquote>
هذا الوقت <strong>تقديري</strong> (ثقة 0.5) — محسوب بالإسقاط من خط عرض معتدل، لأن الغروب الحقيقي لم يحدث.
</blockquote>

<p>
هذه الشفافية ليست ضعفاً — بل هي <strong>أمانة علمية.</strong>
</p>

<br>

---

<br>

<h2>الهدف</h2>

<p>
بولاريس ليس مجرد تطبيق مواقيت.
</p>

<p>
بل هو <strong>محرك فلكي</strong> مصمم ليعطيك:
</p>

<ul>
  <li><strong>وقتاً منطقياً</strong> — حتى في أصعب الظروف الفلكية</li>
  <li><strong>تفسيراً واضحاً</strong> — كيف تم الحساب ولماذا</li>
  <li><strong>ثقة في النتيجة</strong> — رقم صريح يخبرك بمدى دقة كل قيمة</li>
</ul>

<br>

---

<br>

<h2>الخلاصة</h2>

<p>
سواء كنت في مكة أو ستوكهولم أو القطب الشمالي —
</p>

<p>
بولاريس يعطيك جدول صلاة <strong>كامل، مفهوم، وصادق علمياً.</strong>
</p>

<br>

---

<br>

<p align="center">
  <strong>للتوثيق التقني والتشغيل:</strong> <a href="README.md">README.md</a>
</p>

</div>
