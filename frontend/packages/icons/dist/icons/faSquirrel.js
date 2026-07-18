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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquirrel.js
var require_faSquirrel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquirrel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "squirrel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f71a";
    var svgPathData = "M160 296.2c0 2-.7 3.9-2 5.4c-18.7 22.3-30 51-30 82.3c0 70.7 57.3 128 128 128l96 0 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-16 0c10-13.4 16-30 16-48c0-26.2-12.6-49.4-32-64c-13.4-10-30-16-48-16c-9.4 0-18.3 1.6-26.7 4.5c-8.3 2.9-17.5-1.4-20.4-9.8s1.4-17.5 9.8-20.4c11.7-4.1 24.3-6.4 37.3-6.4c17.2 0 33.5 3.9 48 10.8l0-58.8 21.2 0c26.7 0 51.7-13.4 66.6-35.6L512 192c0-53-43-96-96-96l0-20.8C416 69 411 64 404.8 64c-15.2 0-28.7 7.6-36.8 19.2C359.9 71.6 346.4 64 331.2 64C325 64 320 69 320 75.2L320 96c0 54.4-10.5 108.1-30.7 158c-.9-2-1.3-4.2-1.3-6.4L288 144C288 64.5 223.5 0 144 0S0 64.5 0 144s64.5 144 144 144c2.3 0 4.6-.1 6.9-.2c4.9-.2 9.1 3.5 9.1 8.4zM416 144a16 16 0 1 1 0 32 16 16 0 1 1 0-32z";
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
    exports.faSquirrel = exports.definition;
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

// src/_entries/faSquirrel.js
var import_faSquirrel = __toESM(require_faSquirrel());
var export_faSquirrel = import_faSquirrel.definition;
export {
  export_faSquirrel as faSquirrel
};
