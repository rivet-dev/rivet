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

// src/node_modules/@fortawesome/free-brands-svg-icons/faLinode.js
var require_faLinode = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faLinode.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "linode";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f2b8";
    var svgPathData = "M366.3 186.9l-59.5 36.9-.8 36.9-29.3-19.3-39.4 24.3c2.2 55.2 2.5 59.3 2.5 59.5l-97.2 65.4-15.1-104.7 108.1-62-40.2-26-75.4 38.5-21-143.3 129-49.4-91.3-43.6-125.7 39.4 27.7 134.9 41.9 32.7-31.8 15.1 20.9 101.4 29.3 27.7-20.9 12.6 16.8 78.8 66.2 69.6c-10.8-74.8-11.7-78.6-11.7-78.8l77.9-55.3C244 365.4 242.3 367 242.3 367l.8 24.3 33.5 28.5-.8-77.1 46.9-33.5 26.8-18.4-2.5 36 25.1 17.6 6.7-74.6 58.7-43.6-71.2-39.4z";
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
    exports.faLinode = exports.definition;
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

// src/_entries/faLinode.js
var import_faLinode = __toESM(require_faLinode());
var export_faLinode = import_faLinode.definition;
export {
  export_faLinode as faLinode
};
