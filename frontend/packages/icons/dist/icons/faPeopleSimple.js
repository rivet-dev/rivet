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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPeopleSimple.js
var require_faPeopleSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPeopleSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "people-simple";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e21b";
    var svgPathData = "M128 128A64 64 0 1 0 128 0a64 64 0 1 0 0 128zm-22.4 32c-41.6 0-76.3 31.9-79.7 73.4l-4.1 49.3c-2.5 29.8 15.7 56.1 42.2 65.6L64 464c0 26.5 21.5 48 48 48l32 0c26.5 0 48-21.5 48-48l0-115.7c26.5-9.5 44.7-35.8 42.2-65.6l-4.1-49.3C226.7 191.9 192 160 150.4 160l-44.8 0zM384 128A64 64 0 1 0 384 0a64 64 0 1 0 0 128zm-15.7 32c-28.6 0-53.7 18.9-61.5 46.4L267.7 343.2c-5.8 20.4 9.5 40.8 30.8 40.8l21.6 0 0 80c0 26.5 21.5 48 48 48l32 0c26.5 0 48-21.5 48-48l0-80 21.6 0c21.3 0 36.6-20.3 30.8-40.8L461.3 206.4c-7.9-27.5-33-46.4-61.5-46.4l-31.4 0z";
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
    exports.faPeopleSimple = exports.definition;
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

// src/_entries/faPeopleSimple.js
var import_faPeopleSimple = __toESM(require_faPeopleSimple());
var export_faPeopleSimple = import_faPeopleSimple.definition;
export {
  export_faPeopleSimple as faPeopleSimple
};
