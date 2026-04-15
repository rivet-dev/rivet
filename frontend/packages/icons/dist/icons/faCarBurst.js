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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCarBurst.js
var require_faCarBurst = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCarBurst.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-burst";
    var width = 640;
    var height = 512;
    var aliases = ["car-crash"];
    var unicode = "f5e1";
    var svgPathData = "M232 16.1l0-48c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 48c0 13.3 10.7 24 24 24s24-10.7 24-24zM32 168.1l48 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0c-13.3 0-24 10.7-24 24s10.7 24 24 24zM281.5 70.6c9.4 9.4 24.6 9.4 33.9 0l33.9-33.9c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L281.5 36.6c-9.4 9.4-9.4 24.6 0 33.9zm-181 215l33.9-33.9c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L66.6 251.6c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0zM66.6 2.7c-9.4 9.4-9.4 24.6 0 33.9l33.9 33.9c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L100.5 2.7C91.1-6.7 76-6.7 66.6 2.7zM352.9 175.4L505 216.2c6.4 1.7 11.1 7.3 11.8 13.9l7.2 74-231.5-62 43.2-60.5c3.9-5.4 10.7-7.9 17.2-6.2zM223.6 228.5l-2.1 2.9c-21.7 5.5-39.9 22.3-46.1 45.5-4.1 15.5-12.4 46.4-24.8 92.7l-8.3 30.9c-4.6 17.1 5.6 34.6 22.6 39.2l15.5 4.1c17.1 4.6 34.6-5.6 39.2-22.6l8.3-30.9 278.2 74.5-8.3 30.9c-4.6 17.1 5.6 34.6 22.6 39.2l15.5 4.1c17.1 4.6 34.6-5.6 39.2-22.6 4.1-15.5 12.4-46.4 24.8-92.7l8.3-30.9c6.2-23.2-1.1-46.8-17.2-62.5l-.3-3.6-10-103c-3.2-33.2-26.7-60.9-58.9-69.5L369.5 113.6c-32.2-8.6-66.4 3.6-85.8 30.8l-60.2 84.2zm48.7 57.8a32 32 0 1 1 -16.6 61.8 32 32 0 1 1 16.6-61.8zm208.1 88.9a32 32 0 1 1 61.8 16.6 32 32 0 1 1 -61.8-16.6z";
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
    exports.faCarBurst = exports.definition;
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

// src/_entries/faCarBurst.js
var import_faCarBurst = __toESM(require_faCarBurst());
var export_faCarBurst = import_faCarBurst.definition;
export {
  export_faCarBurst as faCarBurst
};
