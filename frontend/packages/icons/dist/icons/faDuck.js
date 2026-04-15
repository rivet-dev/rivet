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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDuck.js
var require_faDuck = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDuck.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "duck";
    var width = 512;
    var height = 512;
    var aliases = [129414];
    var unicode = "f6d8";
    var svgPathData = "M400.7 224c-.4 0-.7 .3-.7 .7l0 24c0 6.6 2.8 12.8 7.2 17.7C427.6 289.1 440 319.1 440 352c0 70.7-57.3 128-128 128l-116 0C87.8 480 0 392.2 0 284l0-5.4C0 266.1 10.1 256 22.6 256c6 0 11.8 2.4 16 6.6L54 278c6.2 6.2 15.2 8.5 23.6 6.1l172.8-49.4c10.4-3 15.7-14.5 7.8-21.7c-21-19-34.1-46.5-34.1-77c0-57.4 46.6-104 104-104c53.8 0 98.1 40.9 103.5 93.3c0 .4 .3 .6 .7 .7c10.5 1.3 21.2 1.3 31.8 0l35.3-4.4c6.8-.8 12.8 4.4 12.8 11.3c0 50.3-40.8 91.1-91.1 91.1l-20.2 0zM368 136a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM340.6 296.9c.5-8.8-6.2-16.4-15-16.9s-16.4 6.2-16.9 15c-2.4 41.2-32.6 79-77.3 90.1c-40.3 10.1-80.5-4.6-103.7-33.5c-5.5-6.9-15.6-8-22.5-2.5s-8 15.6-2.5 22.5c31.1 38.7 84 57.6 136.4 44.5c58-14.5 98.3-63.8 101.5-119.3z";
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
    exports.faDuck = exports.definition;
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

// src/_entries/faDuck.js
var import_faDuck = __toESM(require_faDuck());
var export_faDuck = import_faDuck.definition;
export {
  export_faDuck as faDuck
};
