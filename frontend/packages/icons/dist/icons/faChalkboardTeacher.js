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

// src/node_modules/@fortawesome/free-solid-svg-icons/faChalkboardUser.js
var require_faChalkboardUser = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faChalkboardUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chalkboard-user";
    var width = 640;
    var height = 512;
    var aliases = ["chalkboard-teacher"];
    var unicode = "f51c";
    var svgPathData = "M192 384c53 0 96 43 96 96 0 17.7-14.3 32-32 32L32 512c-17.7 0-32-14.3-32-32 0-53 43-96 96-96l96 0zM544 32c35.3 0 64 28.7 64 64l0 288c0 33.1-25.2 60.4-57.5 63.7l-6.5 .3-211.1 0c-5.1-24.2-16.3-46.1-32.1-64l51.2 0 0-32c0-17.7 14.3-32 32-32l96 0c17.7 0 32 14.3 32 32l0 32 32 0 0-288-352 0 0 57.3c-14.8-6-31-9.3-48-9.3-5.4 0-10.8 .3-16 1l0-49c0-35.3 28.7-64 64-64l352 0zM144 352a80 80 0 1 1 0-160 80 80 0 1 1 0 160z";
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
    exports.faChalkboardUser = exports.definition;
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

// src/_entries/faChalkboardTeacher.js
var import_faChalkboardUser = __toESM(require_faChalkboardUser());
var export_faChalkboardTeacher = import_faChalkboardUser.definition;
export {
  export_faChalkboardTeacher as faChalkboardTeacher
};
