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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserDoctor.js
var require_faUserDoctor = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserDoctor.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-doctor";
    var width = 448;
    var height = 512;
    var aliases = ["user-md"];
    var unicode = "f0f0";
    var svgPathData = "M224 8a120 120 0 1 0 0 240 120 120 0 1 0 0-240zm60 312.8c-5.4-.5-11-.8-16.6-.8l-86.9 0c-5.6 0-11.1 .3-16.6 .8l0 67.5c16.5 7.6 28 24.3 28 43.6 0 26.5-21.5 48-48 48s-48-21.5-48-48c0-19.4 11.5-36.1 28-43.6l0-58.4C61 353 16 413.6 16 484.6 16 499.7 28.3 512 43.4 512l361.1 0c15.1 0 27.4-12.3 27.4-27.4 0-71-45-131.5-108-154.6l0 37.4c23.3 8.2 40 30.5 40 56.6l0 32c0 11-9 20-20 20s-20-9-20-20l0-32c0-11-9-20-20-20s-20 9-20 20l0 32c0 11-9 20-20 20s-20-9-20-20l0-32c0-26.1 16.7-48.3 40-56.6l0-46.6z";
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
    exports.faUserDoctor = exports.definition;
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

// src/_entries/faUserDoctor.js
var import_faUserDoctor = __toESM(require_faUserDoctor());
var export_faUserDoctor = import_faUserDoctor.definition;
export {
  export_faUserDoctor as faUserDoctor
};
