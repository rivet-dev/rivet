var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerLettuce.js
var require_faBurgerLettuce = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerLettuce.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "burger-lettuce";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3e3";
    var svgPathData = "M32 194.9C32 211 45 224 61.1 224l389.8 0c16.1 0 29.1-13 29.1-29.1c0-1.9-.2-3.7-.6-5.6C474.1 168.3 433.2 32 256 32S37.9 168.3 32.6 189.3c-.5 1.9-.6 3.7-.6 5.6zM128 112a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm240 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM256 80a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM32 400l0 16c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-16c0-8.8-7.2-16-16-16L48 384c-8.8 0-16 7.2-16 16zm48-48c24.6 0 44.2-11.8 56.2-19c.9-.5 1.7-1 2.5-1.5c13.7-8.1 20.8-11.5 29.3-11.5c7.7 0 12.3 2.5 25.4 10.9l.2 .1c13 8.3 32.6 21 62.4 21c29.7 0 48.6-12.6 61-20.9l.8-.5c11.6-7.7 16.6-10.6 26.2-10.6c10.5 0 17.9 3.7 30.2 11.3c.9 .5 1.8 1.1 2.8 1.7c11.4 7.1 30.3 19 55 19c21.5 0 38.4-5.4 50.3-11.4c5.9-3 10.5-6 13.9-8.5c1.7-1.3 3.1-2.4 4.1-3.3c.5-.5 1-.9 1.4-1.2l.5-.5 .2-.2 .1-.1c0 0 0 0 0 0c0 0 0 0 0 0L480 304l22.6 22.6c12.5-12.5 12.5-32.8 0-45.3c-12.3-12.3-32-12.5-44.5-.7l-.3 .2c-.6 .5-2 1.4-4.1 2.5c-4.1 2-11.2 4.6-21.7 4.6c-5.2 0-9.2-2-24.2-11.3C394.1 268.3 373.5 256 344 256c-29.7 0-48.6 12.6-61 20.9l-.8 .5C270.6 285.1 265.7 288 256 288c-10 0-16.1-3.2-28-10.9l-1.8-1.1c-11.9-7.7-30.8-20-58.3-20c-27.7 0-48.4 12.3-61.6 20.2l-.5 .3C90.7 285.6 85.6 288 80 288c-10.5 0-17.6-2.6-21.7-4.6c-2.1-1-3.5-2-4.1-2.5l-.3-.2c-12.5-11.8-32.3-11.6-44.5 .7c-12.5 12.5-12.5 32.8 0 45.3l22.4-22.4C9.4 326.6 9.4 326.6 9.4 326.6s0 0 0 0c0 0 0 0 0 0l.1 .1 .2 .2 .5 .5c.4 .4 .8 .8 1.4 1.2c1.1 .9 2.4 2.1 4.1 3.3c3.4 2.5 8 5.6 13.9 8.5C41.6 346.6 58.5 352 80 352z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBurgerLettuce = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBurgerLettuce.js
var import_faBurgerLettuce = __toESM(require_faBurgerLettuce());
var export_faBurgerLettuce = import_faBurgerLettuce.definition;
export {
  export_faBurgerLettuce as faBurgerLettuce
};
