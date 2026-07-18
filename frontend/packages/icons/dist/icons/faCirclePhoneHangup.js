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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCirclePhoneHangup.js
var require_faCirclePhoneHangup = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCirclePhoneHangup.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-phone-hangup";
    var width = 512;
    var height = 512;
    var aliases = ["phone-circle-down"];
    var unicode = "e11d";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM391.4 310c-5.3 8.6-16.8 12.3-26.7 8.5l-46.2-17.6c-8.7-3.3-14.1-11.6-13.1-20.3l2.9-26.7c-33.8-10.8-70.8-10.8-104.6 0l2.9 26.7c.9 8.7-4.4 16.9-13.1 20.3l-46.2 17.6c-9.9 3.8-21.4 .2-26.7-8.5L98.8 274.8c-4.8-7.8-3.4-17.5 3.4-23.8c84.9-78.6 222.6-78.6 307.5 0c6.8 6.3 8.2 16.1 3.4 23.8L391.4 310z";
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
    exports.faCirclePhoneHangup = exports.definition;
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

// src/_entries/faCirclePhoneHangup.js
var import_faCirclePhoneHangup = __toESM(require_faCirclePhoneHangup());
var export_faCirclePhoneHangup = import_faCirclePhoneHangup.definition;
export {
  export_faCirclePhoneHangup as faCirclePhoneHangup
};
