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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUsersViewfinder.js
var require_faUsersViewfinder = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUsersViewfinder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "users-viewfinder";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e595";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64l0 72c0 13.3 10.7 24 24 24s24-10.7 24-24l0-72c0-8.8 7.2-16 16-16l72 0c13.3 0 24-10.7 24-24S149.3 0 136 0L64 0zM440 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0c8.8 0 16 7.2 16 16l0 72c0 13.3 10.7 24 24 24s24-10.7 24-24l0-72c0-35.3-28.7-64-64-64L440 0zM48 376c0-13.3-10.7-24-24-24S0 362.7 0 376l0 72c0 35.3 28.7 64 64 64l72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0c-8.8 0-16-7.2-16-16l0-72zm528 0c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 72c0 8.8-7.2 16-16 16l-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0c35.3 0 64-28.7 64-64l0-72zM288 216a56 56 0 1 0 0-112 56 56 0 1 0 0 112zm0 40c-53 0-96 43-96 96l0 24c0 13.3 10.7 24 24 24l144 0c13.3 0 24-10.7 24-24l0-24c0-53-43-96-96-96zm192-64a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM168 272.3c-49.3 4.1-88 45.3-88 95.7l0 10.7c0 11.8 9.6 21.3 21.3 21.3l46.8 0c-2.7-7.5-4.1-15.6-4.1-24l0-24c0-29.5 8.8-56.9 24-79.7zM427.9 400l46.8 0c11.8 0 21.3-9.6 21.3-21.3l0-10.7c0-50.3-38.7-91.6-88-95.7 15.2 22.8 24 50.2 24 79.7l0 24c0 8.4-1.4 16.5-4.1 24zM192 192a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faUsersViewfinder = exports.definition;
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

// src/_entries/faUsersViewfinder.js
var import_faUsersViewfinder = __toESM(require_faUsersViewfinder());
var export_faUsersViewfinder = import_faUsersViewfinder.definition;
export {
  export_faUsersViewfinder as faUsersViewfinder
};
