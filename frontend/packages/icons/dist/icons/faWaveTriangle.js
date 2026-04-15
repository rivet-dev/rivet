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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWaveTriangle.js
var require_faWaveTriangle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWaveTriangle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wave-triangle";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f89a";
    var svgPathData = "M176 32c10.1 0 19.6 4.7 25.6 12.8L464 394.7 582.4 236.8c10.6-14.1 30.7-17 44.8-6.4s17 30.7 6.4 44.8l-144 192c-6 8.1-15.5 12.8-25.6 12.8s-19.6-4.7-25.6-12.8L176 117.3 57.6 275.2c-10.6 14.1-30.7 17-44.8 6.4s-17-30.7-6.4-44.8l144-192c6-8.1 15.5-12.8 25.6-12.8z";
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
    exports.faWaveTriangle = exports.definition;
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

// src/_entries/faWaveTriangle.js
var import_faWaveTriangle = __toESM(require_faWaveTriangle());
var export_faWaveTriangle = import_faWaveTriangle.definition;
export {
  export_faWaveTriangle as faWaveTriangle
};
