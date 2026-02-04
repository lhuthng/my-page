<section>
  <h1>Privacy Policy</h1>

  <p>I collect and use data as follows:</p>

  <h3>Authentication</h3>
  <p>
    I use <code>cookies</code> to keep you logged in. These are necessary for account
    functionality.
  </p>

  <h3>Engagement Tracking</h3>
  <p>
    I use <code>localStorage</code> to track which posts you have viewed and liked.
    This data is stored locally on your device to prevent spam and ensure accurate
    view counts.
  </p>

  <h3>Data Security</h3>
  <ul>
    <li>I do not sell or share your data with third parties.</li>
    <li>
      Passwords are salted and hashed in my database; I cannot see your actual
      password.
    </li>
  </ul>

  <h3>Data Deletion</h3>
  <p>
    If you want your account or data removed, contact me at
    <a class="text-accent-blue" href="mailto:huuthang.l@outlook.com"
      >huuthang.l@outlook.com</a
    >.
  </p>
</section>

<style lang="postcss">
  @reference "../../app.css";

  section {
    @apply bg-white rounded-xl text-base p-4;
  }

  h1 {
    @apply text-2xl font-semibold;
  }
  h3 {
    @apply pt-4 text-xl font-semibold;
    counter-increment: h3;

    &::before {
      @apply font-semibold;
      content: counter(h3) ". ";
    }
  }
  code {
    @apply px-1 text-base bg-dark/20 rounded-sm;
  }
</style>
