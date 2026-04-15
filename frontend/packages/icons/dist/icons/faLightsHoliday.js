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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightsHoliday.js
var require_faLightsHoliday = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightsHoliday.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lights-holiday";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7b2";
    var svgPathData = "M5.4 78.2c9.8-14.7 29.6-18.7 44.3-9c0 0 0 0 0 0l.4 .3c.5 .3 1.3 .8 2.5 1.5c2.4 1.4 6.3 3.6 11.7 6.3C75 82.7 91.6 90.1 114.1 97.6C159 112.6 227.6 128 320 128s161-15.4 205.9-30.4c22.5-7.5 39.1-14.9 49.8-20.3c5.4-2.7 9.3-4.9 11.7-6.3c1.2-.7 2.1-1.2 2.5-1.5l.4-.2s0 0 0 0c0 0 0 0 .1 0c14.7-9.7 34.5-5.8 44.3 8.9c9.8 14.7 5.8 34.6-8.9 44.4L608 96c17.8 26.6 17.7 26.6 17.7 26.6s0 0 0 0c0 0 0 0-.1 0l-.2 .1-.4 .3c-.3 .2-.7 .5-1.2 .8c-1 .6-2.4 1.5-4.2 2.5c-3.6 2.1-8.7 4.9-15.3 8.2c-13.3 6.6-32.7 15.2-58.2 23.7C495 175.4 419.6 192 320 192s-175-16.6-226.1-33.6c-25.5-8.5-44.9-17.1-58.2-23.7c-6.6-3.3-11.7-6.1-15.3-8.2c-1.8-1-3.2-1.9-4.2-2.5c-.5-.3-.9-.6-1.2-.8l-.4-.3-.1-.1c0 0 0 0-.1 0c0 0 0 0 0 0L32 96 14.2 122.6C-.5 112.8-4.4 93 5.4 78.2zM523.2 198.7l9.4 25.8c30.8 1.9 59 21.7 70.2 52.5c16.4 45.1-8.8 139.4-31.4 147.6s-102.5-47.8-119-92.9c-11.2-30.8-2.3-64.1 20-85.3l-12.3-33.8c23.2-4 44.2-8.7 63.1-13.8zM320 224c10.9 0 21.6-.2 32-.6l0 39.2c28.3 12.3 48 40.5 48 73.3c0 48-56 128-80 128s-80-80-80-128c0-32.8 19.7-61 48-73.3l0-39.2c10.4 .4 21.1 .6 32 .6zM179.9 212.5l-12.3 33.8c22.3 21.3 31.2 54.5 20 85.3c-16.4 45.1-96.4 101.1-119 92.9S20.8 322.1 37.3 277c11.2-30.8 39.4-50.6 70.2-52.5l9.4-25.8c18.8 5.1 39.9 9.9 63.1 13.8z";
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
    exports.faLightsHoliday = exports.definition;
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

// src/_entries/faLightsHoliday.js
var import_faLightsHoliday = __toESM(require_faLightsHoliday());
var export_faLightsHoliday = import_faLightsHoliday.definition;
export {
  export_faLightsHoliday as faLightsHoliday
};
