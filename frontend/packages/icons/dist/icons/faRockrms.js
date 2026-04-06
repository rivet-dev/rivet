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

// src/node_modules/@fortawesome/free-brands-svg-icons/faRockrms.js
var require_faRockrms = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faRockrms.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "rockrms";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f3e9";
    var svgPathData = "M256 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zM413.4 427.5l-90 0-112-131.3c-17.9-20.4-3.9-56.1 26.6-56.1l75.3 0-84.6-99.3-84.3 98.9-90 0 147.1-172.5c14.4-18.4 41.3-17.3 54.5 0L413.7 252.3c19 22.8 2 57.2-27.6 56.1-.6 0-74.2 .2-74.2 .2L413.4 427.5z";
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
    exports.faRockrms = exports.definition;
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

// src/_entries/faRockrms.js
var import_faRockrms = __toESM(require_faRockrms());
var export_faRockrms = import_faRockrms.definition;
export {
  export_faRockrms as faRockrms
};
