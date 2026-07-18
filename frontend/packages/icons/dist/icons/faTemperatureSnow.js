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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTemperatureSnow.js
var require_faTemperatureSnow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTemperatureSnow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "temperature-snow";
    var width = 576;
    var height = 512;
    var aliases = ["temperature-frigid"];
    var unicode = "f768";
    var svgPathData = "M384 112c0-26.5 21.5-48 48-48s48 21.5 48 48l0 164.5c0 17.3 7.1 31.9 15.3 42.5C505.8 332.6 512 349.5 512 368c0 44.2-35.8 80-80 80s-80-35.8-80-80c0-18.5 6.2-35.4 16.7-48.9c8.2-10.6 15.3-25.2 15.3-42.5L384 112zM432 0C370.1 0 320 50.1 320 112l0 164.4c0 .1-.1 .3-.2 .6c-.2 .6-.8 1.6-1.7 2.8c-18.9 24.4-30.1 55-30.1 88.1c0 79.5 64.5 144 144 144s144-64.5 144-144c0-33.2-11.3-63.8-30.1-88.1c-.9-1.2-1.5-2.2-1.7-2.8c-.1-.3-.2-.5-.2-.6L544 112C544 50.1 493.9 0 432 0zm0 416c26.5 0 48-21.5 48-48c0-20.9-13.4-38.7-32-45.3l0-50.7c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 50.7c-18.6 6.6-32 24.4-32 45.3c0 26.5 21.5 48 48 48zM288 131.6l-88 50.8 0-55.8 35.3-35.3c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L200 81.4 200 56c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 25.4L139.3 68.7c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L152 126.6l0 55.8-48.3-27.9L90.7 106.3C88.5 97.7 79.7 92.7 71.2 95s-13.6 11.1-11.3 19.6l4.6 17.3-22-12.7C31 112.6 16.3 116.5 9.7 128s-2.7 26.2 8.8 32.8l22 12.7-17.3 4.6c-8.5 2.3-13.6 11.1-11.3 19.6s11.1 13.6 19.6 11.3l48.2-12.9L128 224 79.7 251.9 31.4 239c-8.5-2.3-17.3 2.8-19.6 11.3s2.8 17.3 11.3 19.6l17.3 4.6-22 12.7C7 293.8 3.1 308.5 9.7 320s21.3 15.4 32.8 8.8l22-12.7-4.6 17.3c-2.3 8.5 2.8 17.3 11.3 19.6s17.3-2.8 19.6-11.3l12.9-48.2L152 265.6l0 55.8-35.3 35.3c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0L152 366.6l0 25.4c0 13.3 10.7 24 24 24s24-10.7 24-24l0-25.4 12.7 12.7c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6L200 321.4l0-55.8 67.7 39.1c5.2-13.5 12-26.2 20.3-37.9l0-5.8-64-37 64-37 0-55.4z";
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
    exports.faTemperatureSnow = exports.definition;
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

// src/_entries/faTemperatureSnow.js
var import_faTemperatureSnow = __toESM(require_faTemperatureSnow());
var export_faTemperatureSnow = import_faTemperatureSnow.definition;
export {
  export_faTemperatureSnow as faTemperatureSnow
};
