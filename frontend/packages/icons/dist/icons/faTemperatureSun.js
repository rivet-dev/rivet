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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTemperatureSun.js
var require_faTemperatureSun = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTemperatureSun.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "temperature-sun";
    var width = 640;
    var height = 512;
    var aliases = ["temperature-hot"];
    var unicode = "f76a";
    var svgPathData = "M320.6 98.5l-2.7-.5L303.8 13.4c-.9-5.4-4.5-10-9.6-12.1s-10.9-1.5-15.4 1.6L208 52.5 137.2 2.9c-4.5-3.2-10.3-3.8-15.4-1.6s-8.7 6.7-9.6 12.1L98.1 98.1 13.4 112.2c-5.4 .9-10 4.5-12.1 9.6s-1.5 10.9 1.6 15.4L52.5 208 2.9 278.8c-3.2 4.5-3.8 10.3-1.6 15.4s6.7 8.7 12.1 9.6l84.7 14.1 14.1 84.7c.9 5.4 4.5 10 9.6 12.1s10.9 1.5 15.4-1.6L208 363.5l70.8 49.6c4.5 3.1 10.3 3.8 15.4 1.6c.7-.3 1.5-.7 2.1-1.1c-5.4-16.9-8.3-34.9-8.3-53.7c0-37.7 11.8-72.6 32-101.2L320 112c0-4.5 .2-9 .6-13.5zM208 272a64 64 0 1 0 0-128 64 64 0 1 0 0 128zm0-160a96 96 0 1 1 0 192 96 96 0 1 1 0-192zm208 0c0-26.5 21.5-48 48-48s48 21.5 48 48l0 164.5c0 17.3 7.1 31.9 15.3 42.5C537.8 332.6 544 349.5 544 368c0 44.2-35.8 80-80 80s-80-35.8-80-80c0-18.5 6.2-35.4 16.7-48.9c8.2-10.6 15.3-25.2 15.3-42.5L416 112zM464 0C402.1 0 352 50.2 352 112l0 164.4c0 .1-.1 .3-.2 .6c-.2 .6-.8 1.6-1.7 2.8c-18.9 24.4-30.1 55-30.1 88.1c0 79.5 64.5 144 144 144s144-64.5 144-144c0-33.2-11.3-63.8-30.1-88.1c-.9-1.2-1.5-2.2-1.7-2.8c-.1-.3-.2-.5-.2-.6L576 112C576 50.2 525.9 0 464 0zm0 416c26.5 0 48-21.5 48-48c0-20.9-13.4-38.7-32-45.3L480 112c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 210.7c-18.6 6.6-32 24.4-32 45.3c0 26.5 21.5 48 48 48z";
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
    exports.faTemperatureSun = exports.definition;
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

// src/_entries/faTemperatureSun.js
var import_faTemperatureSun = __toESM(require_faTemperatureSun());
var export_faTemperatureSun = import_faTemperatureSun.definition;
export {
  export_faTemperatureSun as faTemperatureSun
};
