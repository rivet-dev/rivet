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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSkating.js
var require_faPersonSkating = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSkating.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-skating";
    var width = 512;
    var height = 512;
    var aliases = ["skating"];
    var unicode = "f7c5";
    var svgPathData = "M368 56a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM128 128c0-17.7 14.3-32 32-32l150.1 0c49.9 0 74.9 60.3 39.6 95.6l-63.9 63.9 69.8 31c17.3 7.7 28.5 24.9 28.5 43.9l0 85.6c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-75.2-82.4-36.6c-41-18.2-51-72-19.3-103.7l40.4-40.4-98.7 0c-17.7 0-32-14.3-32-32zM89.4 361.4l75.9-75.9c7.7 19.7 21.6 37.3 41 49.6l-71.6 71.6c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3zm346.5 87.1c8.6 2.1 13.8 10.8 11.6 19.4l-.4 1.7c-6.2 24.9-28.6 42.4-54.3 42.4L304 512c-8.8 0-16-7.2-16-16s7.2-16 16-16l88.8 0c11 0 20.6-7.5 23.3-18.2l.4-1.7c2.1-8.6 10.8-13.8 19.4-11.6zM143.2 486.3l-6.2 3.1c-21.6 10.8-47.6 6.6-64.6-10.5L12.7 419.3c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0l59.6 59.6c7.3 7.3 18.5 9.1 27.7 4.5l6.2-3.1c7.9-4 17.5-.7 21.5 7.2s.7 17.5-7.2 21.5z";
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
    exports.faPersonSkating = exports.definition;
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

// src/_entries/faSkating.js
var import_faPersonSkating = __toESM(require_faPersonSkating());
var export_faSkating = import_faPersonSkating.definition;
export {
  export_faSkating as faSkating
};
