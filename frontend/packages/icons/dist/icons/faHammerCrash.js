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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHammerCrash.js
var require_faHammerCrash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHammerCrash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hammer-crash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e414";
    var svgPathData = "M283.9 378.6l18.3-60.1c18-4.1 34.2-16 43.1-33.8l64-128c10.5-21.1 8.4-45.2-3.7-63.6l52.7-76.6c3.7-5.4 10.4-8 16.7-6.5s11.2 6.7 12.2 13.1l16.2 104.1 105.1-7.4c6.5-.5 12.7 3.1 15.5 9s1.8 12.9-2.6 17.8L550.1 224l71.3 77.5c4.4 4.8 5.5 11.9 2.6 17.8s-9 9.5-15.5 9l-105.1-7.4L487.3 425c-1 6.5-5.9 11.7-12.2 13.1s-13-1.1-16.7-6.5l-59.7-86.7-91.4 52.2c-5.7 3.3-12.8 2.7-17.9-1.4s-7.2-10.9-5.3-17.2zm28.3-101.7c-9.3 10.9-25.2 14.4-38.6 7.7l-65.9-32.9s0 0 0 0L122 208.8s0 0 0 0L17.7 156.6C1.9 148.7-4.5 129.5 3.4 113.7l40-80C48.8 22.8 59.9 16 72 16l120 0c5 0 9.9 1.2 14.3 3.4l78.2 39.1 81.8 40.9c15.8 7.9 22.2 27.1 14.3 42.9l-64 128c-1.2 2.4-2.7 4.6-4.4 6.6zM107.6 237.4l85.9 42.9L90.9 485.5c-11.9 23.7-40.7 33.3-64.4 21.5S-6.8 466.2 5.1 442.5L107.6 237.4z";
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
    exports.faHammerCrash = exports.definition;
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

// src/_entries/faHammerCrash.js
var import_faHammerCrash = __toESM(require_faHammerCrash());
var export_faHammerCrash = import_faHammerCrash.definition;
export {
  export_faHammerCrash as faHammerCrash
};
