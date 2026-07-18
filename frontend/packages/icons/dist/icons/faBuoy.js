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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBuoy.js
var require_faBuoy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBuoy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "buoy";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5b5";
    var svgPathData = "M192 0c-17.7 0-32 14.3-32 32c0 15.3 10.8 28.1 25.1 31.3L133.8 320 96 320c-17.7 0-32 14.3-32 32l0 16c0 8.2 .7 16.1 2 23.9c21-11.1 46.9-9.9 66.9 4.2c18 12.4 40.1 20.3 59.2 20.3c21.1 0 42-8.5 59.2-20.3c22.1-15.5 51.6-15.5 73.7 0c18.4 12.7 39.6 20.3 59.2 20.3c19 0 41.2-7.9 59.2-20.3c20-14.1 45.9-15.3 66.9-4.2c1.3-7.8 2-15.8 2-23.9l0-16c0-17.7-14.3-32-32-32l-37.8 0L390.9 63.3C405.2 60.1 416 47.3 416 32c0-17.7-14.3-32-32-32L192 0zM325.8 64l9.8 49L288 168.5 240.4 113l9.8-49 75.5 0zm21.8 108.8l17 84.9-44.9-52.4 27.9-32.6zm-91.1 32.6l-44.9 52.4 17-84.9 27.9 32.6zM221.3 320L288 242.2 354.7 320l-133.4 0zm85.2 101.9c-11.1-7.9-25.9-7.9-37 0C247 437.4 219.5 448 192 448c-26.9 0-55.3-10.8-77.4-26.1c0 0 0 0 0 0c-11.9-8.5-28.1-7.8-39.2 1.7c-14.4 11.9-32.5 21-50.6 25.2c-17.2 4-27.9 21.2-23.9 38.4s21.2 27.9 38.4 23.9c24.5-5.7 44.9-16.5 58.2-25C126.5 501.7 159 512 192 512c31.9 0 60.6-9.9 80.4-18.9c5.8-2.7 11.1-5.3 15.6-7.7c4.5 2.4 9.7 5.1 15.6 7.7c19.8 9 48.5 18.9 80.4 18.9c33 0 65.5-10.3 94.5-25.8c13.4 8.4 33.7 19.3 58.2 25c17.2 4 34.4-6.7 38.4-23.9s-6.7-34.4-23.9-38.4c-18.1-4.2-36.2-13.3-50.6-25.2c-11.1-9.4-27.3-10.1-39.2-1.7c0 0 0 0 0 0C439.4 437.2 410.9 448 384 448c-27.5 0-55-10.6-77.5-26.1z";
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
    exports.faBuoy = exports.definition;
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

// src/_entries/faBuoy.js
var import_faBuoy = __toESM(require_faBuoy());
var export_faBuoy = import_faBuoy.definition;
export {
  export_faBuoy as faBuoy
};
