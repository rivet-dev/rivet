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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStackExchange.js
var require_faStackExchange = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStackExchange.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "stack-exchange";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f18d";
    var svgPathData = "M18.1 332.3l412.7 0 0 22c0 37.7-29.3 68-65.3 68l-19 0-86.8 89.7 0-89.7-176.3 0c-36 0-65.3-30.3-65.3-68l0-22zm0-23.6l412.7 0 0-85-412.7 0 0 85zm0-109.4l412.7 0 0-85-412.7 0 0 85zM365.4 0L83.4 0c-36 0-65.3 30.3-65.3 67.7l0 22.3 412.7 0 0-22.3C430.7 30.3 401.4 0 365.4 0z";
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
    exports.faStackExchange = exports.definition;
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

// src/_entries/faStackExchange.js
var import_faStackExchange = __toESM(require_faStackExchange());
var export_faStackExchange = import_faStackExchange.definition;
export {
  export_faStackExchange as faStackExchange
};
