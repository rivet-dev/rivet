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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarBus.js
var require_faCarBus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarBus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-bus";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f85a";
    var svgPathData = "M32 416l16 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 64 0 0-64c0-32.8 16.5-61.8 41.6-79.1L249.9 224 208 224l0-128 88 0c13.3 0 24 10.7 24 24l0 16.4c13.2-5.4 27.7-8.4 42.6-8.4l21.4 0 0-53.5c0-16.2-9.2-31-23.7-38.3C312.8 12.4 260.3 0 207.1 0L176.9 0C123.7 0 71.2 12.4 23.7 36.2C9.2 43.4 0 58.2 0 74.5L0 384c0 17.7 14.3 32 32 32zM88 96l88 0 0 128-88 0c-13.3 0-24-10.7-24-24l0-80c0-13.3 10.7-24 24-24zm8 200a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm266.6-72l138.8 0c6.9 0 13 4.4 15.2 10.9L534.3 288l-204.5 0 17.7-53.1c2.2-6.5 8.3-10.9 15.2-10.9zm-75.9-9.3l-26.6 79.7C238.8 304.7 224 326.6 224 352l0 96s0 0 0 0l0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 256 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 0-96c0-25.4-14.8-47.3-36.1-57.6l-26.6-79.7C566.4 182 535.8 160 501.4 160l-138.8 0c-34.4 0-65 22-75.9 54.7zM304 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm232 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faCarBus = exports.definition;
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

// src/_entries/faCarBus.js
var import_faCarBus = __toESM(require_faCarBus());
var export_faCarBus = import_faCarBus.definition;
export {
  export_faCarBus as faCarBus
};
