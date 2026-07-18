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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendarCircleUser.js
var require_faCalendarCircleUser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendarCircleUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "calendar-circle-user";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e471";
    var svgPathData = "M128 0C110.3 0 96 14.3 96 32l0 32L48 64C21.5 64 0 85.5 0 112l0 48 448 0 0-48c0-26.5-21.5-48-48-48l-48 0 0-32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 32L160 64l0-32c0-17.7-14.3-32-32-32zM256 368c0-91.8 70.3-167.2 160-175.3l0-.7L0 192 0 464c0 26.5 21.5 48 48 48l282.8 0C285.6 480.1 256 427.5 256 368zm320 0a144 144 0 1 0 -288 0 144 144 0 1 0 288 0zm-66.3 80.7C489.5 468.1 462.2 480 432 480s-57.5-11.9-77.7-31.3c6.2-19 24-32.7 45.1-32.7l65.2 0c21 0 38.9 13.7 45.1 32.7zM384 336a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
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
    exports.faCalendarCircleUser = exports.definition;
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

// src/_entries/faCalendarCircleUser.js
var import_faCalendarCircleUser = __toESM(require_faCalendarCircleUser());
var export_faCalendarCircleUser = import_faCalendarCircleUser.definition;
export {
  export_faCalendarCircleUser as faCalendarCircleUser
};
