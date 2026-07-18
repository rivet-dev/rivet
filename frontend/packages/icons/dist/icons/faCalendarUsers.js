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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendarUsers.js
var require_faCalendarUsers = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCalendarUsers.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "calendar-users";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5e2";
    var svgPathData = "M192 32l0 32-48 0c-26.5 0-48 21.5-48 48l0 48 32 0 384 0 32 0 0-48c0-26.5-21.5-48-48-48l-48 0 0-32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 32L256 64l0-32c0-17.7-14.3-32-32-32s-32 14.3-32 32zm55.4 480l145.1 0c-5.4-9.4-8.6-20.3-8.6-32c0-43.8 29.4-80.8 69.6-92.3c-23-20.5-37.6-50.4-37.6-83.7c0-56.4 41.7-103.1 96-110.9l0-1.1-384 0 0 1.1c54.3 7.8 96 54.4 96 110.9c0 33.3-14.5 63.2-37.6 83.7C226.6 399.2 256 436.2 256 480c0 11.7-3.1 22.6-8.6 32zM528 384a80 80 0 1 0 0-160 80 80 0 1 0 0 160zm-48 32c-35.3 0-64 28.7-64 64c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-96 0zM192 304A80 80 0 1 0 32 304a80 80 0 1 0 160 0zM0 480c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-96 0c-35.3 0-64 28.7-64 64z";
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
    exports.faCalendarUsers = exports.definition;
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

// src/_entries/faCalendarUsers.js
var import_faCalendarUsers = __toESM(require_faCalendarUsers());
var export_faCalendarUsers = import_faCalendarUsers.definition;
export {
  export_faCalendarUsers as faCalendarUsers
};
