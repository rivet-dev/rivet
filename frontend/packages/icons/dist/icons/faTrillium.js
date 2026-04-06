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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrillium.js
var require_faTrillium = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrillium.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trillium";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e588";
    var svgPathData = "M128 160c0-64.1 61.6-128.2 86.1-151.1c5.6-5.3 14.1-5.3 19.8 0c24.5 23 86.1 87 86.1 151.1c0 15.7-3.7 30.4-10.4 43.5c16.2 0 32.6 4.2 47.7 12.9c55.5 32 80.2 117.4 87.8 150.2c1.8 7.5-2.5 14.9-9.9 17.1c-32.1 9.7-118.4 31.1-173.9-1c-16.6-9.6-29.2-23.3-37.4-39.2c-8.1 15.9-20.8 29.6-37.4 39.2c-55.5 32-141.8 10.7-173.9 1c-7.4-2.2-11.6-9.6-9.9-17.1c7.6-32.7 32.3-118.1 87.8-150.2c15.1-8.7 31.6-12.9 47.8-12.9C131.7 190.4 128 175.7 128 160zm136 80a40 40 0 1 0 -80 0 40 40 0 1 0 80 0zm88-80s0 0 0 0c0-12.5-1.8-24.8-4.8-36.6c38.6-12.8 68.2-11.9 83-10c14.9 1.9 23.7 17.1 17.9 31c-5.6 13.3-19 38.4-47.7 64.6c-8.1-7.7-17.1-14.6-27-20.3c0 0 0 0 0 0c-7.3-4.2-14.9-7.6-22.6-10.3c.9-6 1.3-12.1 1.3-18.4zM100.8 123.4c-3 11.8-4.8 24.1-4.8 36.6c0 6.2 .4 12.3 1.3 18.3c-7.8 2.7-15.4 6.1-22.7 10.3c0 0 0 0 0 0c-9.9 5.7-18.8 12.6-26.9 20.2C19 182.7 5.6 157.6 0 144.3c-5.8-13.8 3-29.1 17.9-31c14.7-1.9 44.4-2.8 83 10zm73.3 299.4c9.8-3.1 19.4-7.2 28.5-12.5c7.8-4.5 14.9-9.7 21.4-15.4c6.5 5.8 13.6 11 21.4 15.4c9.2 5.3 18.8 9.4 28.6 12.5c-8.2 36.1-22.6 61.6-31 74.2c-9.1 13.7-28.8 13.7-37.9 0c-8.3-12.6-22.8-38.2-31-74.3z";
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
    exports.faTrillium = exports.definition;
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

// src/_entries/faTrillium.js
var import_faTrillium = __toESM(require_faTrillium());
var export_faTrillium = import_faTrillium.definition;
export {
  export_faTrillium as faTrillium
};
