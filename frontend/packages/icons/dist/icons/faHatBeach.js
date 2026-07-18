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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHatBeach.js
var require_faHatBeach = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHatBeach.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-beach";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e606";
    var svgPathData = "M320 448c176.7 0 320-57.3 320-128c0-19.3-11.8-36.7-29.7-43.9L574.9 262c.7 4.2 1.1 8.5 1.1 12.8c0 42.7-34.6 77.3-77.3 77.3c-12.2 0-24.1-2.9-34.7-8.3c-10.7 5.4-22.6 8.3-34.7 8.3c-38.6 0-70.6-28.3-76.3-65.4c-10.6 .9-21.6 1.4-32.9 1.4c-88.4 0-160-28.7-160-64c0 0 0 0 0 0L29.7 276.1C11.8 283.3 0 300.7 0 320c0 70.7 143.3 128 320 128zm32-242.7c0-42.7 34.6-77.3 77.3-77.3c7.1 0 14.1 1 20.9 2.9C421.1 90.4 373.6 64 320 64c-70.4 0-130.2 45.5-151.6 108.7l.1-.2C174.7 201.3 240.1 224 320 224c11.7 0 23-.5 34-1.4c-1.3-5.6-2-11.5-2-17.3zM473.1 168c-5.5 3.8-12.7 3.8-18.2 0c-7.5-5.2-16.4-8-25.7-8c-25 0-45.3 20.3-45.3 45.3c0 9.2 2.8 18.2 8 25.7c3.8 5.5 3.8 12.7 0 18.2c-5.2 7.5-8 16.4-8 25.7c0 25 20.3 45.3 45.3 45.3c9.2 0 18.2-2.8 25.7-8c5.5-3.8 12.7-3.8 18.2 0c7.5 5.2 16.4 8 25.7 8c25 0 45.3-20.3 45.3-45.3c0-9.2-2.8-18.2-8-25.7c-3.8-5.5-3.8-12.7 0-18.2c5.2-7.5 8-16.4 8-25.7c0-25-20.3-45.3-45.3-45.3c-9.2 0-18.2 2.8-25.7 8zM464 216a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faHatBeach = exports.definition;
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

// src/_entries/faHatBeach.js
var import_faHatBeach = __toESM(require_faHatBeach());
var export_faHatBeach = import_faHatBeach.definition;
export {
  export_faHatBeach as faHatBeach
};
