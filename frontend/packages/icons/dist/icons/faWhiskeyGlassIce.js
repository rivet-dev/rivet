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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWhiskeyGlassIce.js
var require_faWhiskeyGlassIce = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWhiskeyGlassIce.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "whiskey-glass-ice";
    var width = 512;
    var height = 512;
    var aliases = ["glass-whiskey-rocks"];
    var unicode = "f7a1";
    var svgPathData = "M32 32c-9.3 0-18.1 4-24.2 11.1S-1 59.4 .3 68.6l50 342.9c5.7 39.3 39.4 68.5 79.2 68.5l253 0c39.7 0 73.4-29.1 79.2-68.5l50-342.9c1.3-9.2-1.4-18.5-7.5-25.5S489.3 32 480 32L32 32zM417 283.7c9.6 9.6 15 22.7 15 36.3s-5.4 26.7-15 36.3L356.3 417c-9.6 9.6-22.7 15-36.3 15s-26.7-5.4-36.3-15l-49-49L160 368c-35.3 0-64-28.7-64-64l0-22.9L69 96l374 0L415.8 282.5l1.2 1.2zM160 208c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l50.6 0c-1.7-5.1-2.6-10.5-2.6-16c0-13.6 5.4-26.7 15-36.3l33-33 0-10.7c0-17.7-14.3-32-32-32l-64 0zm85.7 98.3C242 310 240 314.9 240 320s2 10 5.7 13.7l60.7 60.7c3.6 3.6 8.5 5.7 13.7 5.7s10-2 13.7-5.7l60.7-60.7c3.6-3.6 5.7-8.5 5.7-13.7s-2-10-5.7-13.7l-60.7-60.7C330 242 325.1 240 320 240s-10 2-13.7 5.7l-60.7 60.7z";
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
    exports.faWhiskeyGlassIce = exports.definition;
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

// src/_entries/faWhiskeyGlassIce.js
var import_faWhiskeyGlassIce = __toESM(require_faWhiskeyGlassIce());
var export_faWhiskeyGlassIce = import_faWhiskeyGlassIce.definition;
export {
  export_faWhiskeyGlassIce as faWhiskeyGlassIce
};
