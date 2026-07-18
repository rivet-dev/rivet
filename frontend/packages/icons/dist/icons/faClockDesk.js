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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClockDesk.js
var require_faClockDesk = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClockDesk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clock-desk";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e134";
    var svgPathData = "M0 224a224 224 0 1 1 448 0A224 224 0 1 1 0 224zM224 96c-13.3 0-24 10.7-24 24l0 104c0 6.4 2.5 12.5 7 17l48 48c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-41-41 0-94.1c0-13.3-10.7-24-24-24zm0 384c96.4 0 180.3-53.3 224-132l0 116c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48L0 348c43.7 78.7 127.6 132 224 132z";
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
    exports.faClockDesk = exports.definition;
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

// src/_entries/faClockDesk.js
var import_faClockDesk = __toESM(require_faClockDesk());
var export_faClockDesk = import_faClockDesk.definition;
export {
  export_faClockDesk as faClockDesk
};
