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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSeatReclined.js
var require_faPersonSeatReclined = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSeatReclined.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-seat-reclined";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e21f";
    var svgPathData = "M176 128a48 48 0 1 0 0-96 48 48 0 1 0 0 96zm89 160l-8-32 63 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-79.2 0c-6.6-19-24.5-32-44.9-32c-30.9 0-53.6 29.1-46.1 59l21.1 84.5C178 332 203.6 352 233 352l6.3 0c.5 0 .9 0 1.4 0l78.1 0c6.7 0 12.6 4.1 15 10.4L370 459.2c6 16.1 23.8 24.6 40.1 19.1l48-16c16.8-5.6 25.8-23.7 20.2-40.5s-23.7-25.8-40.5-20.2l-18.7 6.2-25.5-68c-11.7-31.2-41.6-51.9-74.9-51.9L265 288zM95.5 154.1c-3.3-17.4-20-28.8-37.3-25.6s-28.8 20-25.6 37.3L65.9 343.6C77.2 404.1 130.1 448 191.7 448l80.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-80.3 0c-30.8 0-57.2-21.9-62.9-52.2L95.5 154.1z";
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
    exports.faPersonSeatReclined = exports.definition;
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

// src/_entries/faPersonSeatReclined.js
var import_faPersonSeatReclined = __toESM(require_faPersonSeatReclined());
var export_faPersonSeatReclined = import_faPersonSeatReclined.definition;
export {
  export_faPersonSeatReclined as faPersonSeatReclined
};
