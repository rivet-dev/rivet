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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKidneys.js
var require_faKidneys = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKidneys.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kidneys";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5fb";
    var svgPathData = "M152 0c-21.5 0-45.5 6.3-64.3 23.8C56.4 52.8 0 119.6 0 224c0 24.9 3.5 54.7 7.1 79.1c7.1 48 48.8 80.9 95.3 80.9c58.4 0 104-50.4 98.2-108.5l-4.1-41.2c-2-20.1 4.5-40.1 17.9-55.1l18.2-20.5c15.2-17.1 23.5-39.1 23.5-61.9c0-31.2-15.6-60.2-41.5-77.5l-3.4-2.3C194.5 5.9 175.1 0 155.2 0L152 0zM488 0l-3.2 0c-19.9 0-39.4 5.9-55.9 16.9l-3.4 2.3C399.6 36.5 384 65.6 384 96.7c0 22.8 8.4 44.8 23.5 61.9l18.2 20.5c13.4 15.1 19.9 35.1 17.9 55.1l-4.1 41.2C433.7 333.6 479.3 384 537.7 384c46.5 0 88.2-32.9 95.3-80.9c3.6-24.4 7.1-54.2 7.1-79.1c0-104.4-56.4-171.2-87.7-200.2C533.5 6.3 509.5 0 488 0zM228.2 231.1l1.9 18.8 21.5 10.7c2.7 1.4 4.4 4.1 4.4 7.2L256 488c0 13.3 10.7 24 24 24s24-10.7 24-24l0-220.2c0-21.2-12-40.6-31-50.1l-34.8-17.4-.1 .1c-7.5 8.4-11.1 19.5-10 30.7zm173.6-30.7l-.1-.1L367 217.7c-19 9.5-31 28.9-31 50.1L336 488c0 13.3 10.7 24 24 24s24-10.7 24-24l0-220.2c0-3 1.7-5.8 4.4-7.2l21.5-10.7 1.9-18.8c1.1-11.2-2.5-22.3-10-30.7z";
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
    exports.faKidneys = exports.definition;
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

// src/_entries/faKidneys.js
var import_faKidneys = __toESM(require_faKidneys());
var export_faKidneys = import_faKidneys.definition;
export {
  export_faKidneys as faKidneys
};
