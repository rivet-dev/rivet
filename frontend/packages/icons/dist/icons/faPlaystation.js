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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPlaystation.js
var require_faPlaystation = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPlaystation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "playstation";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f3df";
    var svgPathData = "M571 372.3c-11.3 14.2-38.8 24.3-38.8 24.3l-205.1 73.6 0-54.3 150.9-53.8c17.1-6.1 19.8-14.8 5.8-19.4-13.9-4.6-39.1-3.3-56.2 2.9l-100.5 35.5 0-56.4c23.2-7.8 47.1-13.6 75.7-16.8 40.9-4.5 90.9 .6 130.2 15.5 44.2 14 49.2 34.7 38 48.9zM346.6 279.8l0-139c0-16.3-3-31.3-18.3-35.6-11.7-3.8-19 7.1-19 23.4l0 347.9-93.8-29.8 0-414.7c39.9 7.4 98 24.9 129.2 35.4 79.5 27.3 106.4 61.3 106.4 137.8 0 74.5-46 102.8-104.5 74.6zM43.3 410.2c-45.4-12.8-53-39.5-32.3-54.8 19.1-14.2 51.7-24.9 51.7-24.9l134.5-47.8 0 54.5-96.8 34.6c-17.1 6.1-19.7 14.8-5.8 19.4s39.1 3.3 56.2-2.9l46.4-16.9 0 48.8c-51.6 9.3-101.4 7.3-153.9-10z";
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
    exports.faPlaystation = exports.definition;
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

// src/_entries/faPlaystation.js
var import_faPlaystation = __toESM(require_faPlaystation());
var export_faPlaystation = import_faPlaystation.definition;
export {
  export_faPlaystation as faPlaystation
};
